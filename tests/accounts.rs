//! Accounts: admin and tenant login, token validation, and the tenant lookup.
mod common;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    routing::get,
};
use chrono::Utc;
use common::{
    ADMIN_PASSWORD, ADMIN_USERNAME, LOGIN_LIMIT, TestApp, seed_room, seed_tenant, test_app,
};
use m18_residences_db::entities::tenant;
use m18_residences_server::accounts::repository::tenant_read_repo;
use m18_residences_shared_rs::{
    auth::{Admin, AuthUser, Claims, JwtKeys},
    captcha::FakeCaptcha,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, IntoActiveModel, Set};
use serde_json::{Value, json};
use tower::ServiceExt;

const TENANT_NAME: &str = "Juan Dela Cruz";
// JSON keys the Flutter apps read from the login's tenant.
const TENANT_KEYS: &[&str] = &["id", "room_id", "name", "join_date", "is_active"];

fn now() -> i64 {
    Utc::now().timestamp()
}

fn token_of(body: &Value) -> String {
    body["token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .unwrap_or_else(|| panic!("missing token in {body}"))
        .to_string()
}

/// The claims of a token this app issued.
fn claims_of(app: &TestApp, token: &str) -> Claims {
    app.state.jwt.verify(token).expect("a valid token")
}

fn assert_expires_in(claims: &Claims, seconds: i64) {
    let exp = claims.exp as i64;
    let expected = now() + seconds;
    assert!(
        (expected - 5..=expected).contains(&exp),
        "exp {exp}, expected about {expected}"
    );
}

async fn admin_login(app: &TestApp, username: &str, password: &str) -> (StatusCode, Value) {
    app.post(
        "/api/auth/admin-login",
        None,
        json!({ "username": username, "password": password, "turnstile_token": FakeCaptcha::VALID_TOKEN }),
    )
    .await
}

async fn tenant_login(app: &TestApp, name: &str) -> (StatusCode, Value) {
    app.post(
        "/api/auth/login",
        None,
        json!({ "name": name, "turnstile_token": FakeCaptcha::VALID_TOKEN }),
    )
    .await
}

/// `POST /api/auth/validate-token` with a raw `Authorization` header value.
async fn validate(app: &TestApp, authorization: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/validate-token");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = app
        .router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

async fn seed_juan(app: &TestApp) -> tenant::Model {
    let room = seed_room(app.db(), "Room 101", 5000).await;
    seed_tenant(app.db(), room.id, TENANT_NAME).await
}

// ---- admin login ----

#[tokio::test]
async fn admin_login_returns_token_and_rejects_wrong_password() {
    let app = test_app().await;

    let (status, body) = admin_login(&app, ADMIN_USERNAME, ADMIN_PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!token_of(&body).is_empty());
    assert_eq!(body["role"], "admin");
    assert_eq!(body["username"], ADMIN_USERNAME);

    let (status, body) =
        admin_login(&app, ADMIN_USERNAME, &format!("{ADMIN_PASSWORD}-wrong")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "Invalid credentials" }));
}

#[tokio::test]
async fn admin_login_issues_one_hour_admin_claims_in_wire_order() {
    let app = test_app().await;
    let body = json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "turnstile_token": FakeCaptcha::VALID_TOKEN }).to_string();
    let reply = app
        .request(
            Method::POST,
            "/api/auth/admin-login",
            None,
            Some(("application/json", body.into_bytes())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let text = String::from_utf8(reply.body.clone()).unwrap();
    assert!(text.starts_with(r#"{"token":""#), "{text}");
    let tail = format!(r#","role":"admin","username":"{ADMIN_USERNAME}"}}"#);
    assert!(text.ends_with(&tail), "{text}");

    let claims = claims_of(&app, &token_of(&reply.json()));
    assert_eq!(claims.id, None);
    assert_eq!(claims.name.as_deref(), Some(ADMIN_USERNAME));
    assert_eq!(claims.role.as_deref(), Some("admin"));
    assert_expires_in(&claims, 3600);
}

#[tokio::test]
async fn admin_login_matches_credentials_exactly() {
    let app = test_app().await;
    let expected = json!({ "error": "Invalid credentials" });
    let upper_user = ADMIN_USERNAME.to_uppercase();
    let spaced_user = format!(" {ADMIN_USERNAME}");
    let upper_password = ADMIN_PASSWORD.to_uppercase();
    for (username, password) in [
        ("someone-else", ADMIN_PASSWORD),
        (upper_user.as_str(), ADMIN_PASSWORD),
        (spaced_user.as_str(), ADMIN_PASSWORD),
        (ADMIN_USERNAME, upper_password.as_str()),
        ("", ""),
    ] {
        let (status, body) = admin_login(&app, username, password).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{username:?} / {password:?}"
        );
        assert_eq!(body, expected);
    }
}

// ---- tenant login ----

#[tokio::test]
async fn tenant_login_returns_tenant_and_token() {
    let app = test_app().await;
    let juan = seed_juan(&app).await;

    let (status, body) = tenant_login(&app, TENANT_NAME).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = token_of(&body);
    for key in TENANT_KEYS {
        assert!(
            body["tenant"].get(key).is_some(),
            "tenant login.tenant lacks {key}: {body}"
        );
    }
    assert_eq!(body["tenant"], serde_json::to_value(&juan).unwrap());

    let (status, body) = tenant_login(&app, "Unknown Tenant").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "Tenant not found" }));

    // The token opens that tenant's own records (and only those), as the
    // property and billing handlers check them.
    let claims = claims_of(&app, &token);
    assert!(claims.ensure_admin_or_tenant(juan.id).is_ok());
    assert!(claims.ensure_admin_or_tenant(juan.id + 1).is_err());
}

#[tokio::test]
async fn tenant_login_issues_twenty_minute_tenant_claims() {
    let app = test_app().await;
    let juan = seed_juan(&app).await;
    let body =
        json!({ "name": TENANT_NAME, "turnstile_token": FakeCaptcha::VALID_TOKEN }).to_string();
    let reply = app
        .request(
            Method::POST,
            "/api/auth/login",
            None,
            Some(("application/json", body.into_bytes())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    // Keys are alphabetical, as before: tenant before token.
    let text = String::from_utf8(reply.body.clone()).unwrap();
    assert!(text.starts_with(r#"{"tenant":{"created_at":"#), "{text}");

    let claims = claims_of(&app, &token_of(&reply.json()));
    assert_eq!(claims.id, Some(juan.id));
    assert_eq!(claims.name.as_deref(), Some(TENANT_NAME));
    assert_eq!(claims.role, None);
    assert_expires_in(&claims, 1200);
}

#[tokio::test]
async fn tenant_login_ignores_case_and_surrounding_spaces() {
    let app = test_app().await;
    let juan = seed_juan(&app).await;
    for name in [
        "juan dela cruz",
        "JUAN DELA CRUZ",
        "Juan Dela Cruz ",
        " Juan Dela Cruz",
    ] {
        let (status, body) = tenant_login(&app, name).await;
        assert_eq!(status, StatusCode::OK, "{name:?}: {body}");
        // The token carries the stored name, which the name-keyed routes compare.
        assert_eq!(
            claims_of(&app, &token_of(&body)).name,
            Some(juan.name.clone())
        );
    }
    for name in ["Juan", "Juan  Dela Cruz", ""] {
        let (status, body) = tenant_login(&app, name).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{name:?}");
        assert_eq!(body, json!({ "error": "Tenant not found" }));
    }
}

#[tokio::test]
async fn names_differing_only_by_case_log_in_only_exactly() {
    let app = test_app().await;
    let room = seed_room(app.db(), "Room 101", 5000).await;
    // Only possible in data from before the case-insensitive index (migration 0006).
    app.db()
        .execute_unprepared("DROP INDEX tenants_name_key")
        .await
        .unwrap();
    let upper = seed_tenant(app.db(), room.id, "ANA").await;
    seed_tenant(app.db(), room.id, "Ana").await;

    let (status, body) = tenant_login(&app, "ANA").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tenant"]["id"], upper.id);
    let (status, _) = tenant_login(&app, "ana").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn inactive_tenants_can_still_log_in() {
    let app = test_app().await;
    let juan = seed_juan(&app).await;
    let mut inactive = juan.into_active_model();
    inactive.is_active = Set(false);
    let inactive = inactive.update(app.db()).await.unwrap();

    let (status, body) = tenant_login(&app, TENANT_NAME).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["tenant"]["is_active"], false);
    assert_eq!(body["tenant"], serde_json::to_value(&inactive).unwrap());
}

#[tokio::test]
async fn tenant_login_database_failure_is_a_generic_500() {
    let app = test_app().await;
    app.db()
        .execute_unprepared("ALTER TABLE tenant RENAME TO tenant_gone")
        .await
        .unwrap();

    let (status, body) = tenant_login(&app, TENANT_NAME).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // Intentional change: the database error text is logged, not sent.
    assert_eq!(body, json!({ "error": "Internal server error" }));
}

// ---- malformed bodies (Axum's plain-text rejections, as before) ----

#[tokio::test]
async fn malformed_login_bodies_are_rejected() {
    let app = test_app().await;
    seed_juan(&app).await;
    let json_type = "application/json";
    let valid = r#"{"name":"Juan Dela Cruz","username":"x","password":"x"}"#;
    for path in ["/api/auth/admin-login", "/api/auth/login"] {
        let cases: [(Option<&str>, &str, StatusCode); 7] = [
            (None, valid, StatusCode::UNSUPPORTED_MEDIA_TYPE),
            (
                Some("text/plain"),
                valid,
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ),
            (Some(json_type), "", StatusCode::BAD_REQUEST),
            (Some(json_type), r#"{"name":"#, StatusCode::BAD_REQUEST),
            (Some(json_type), "{}", StatusCode::UNPROCESSABLE_ENTITY),
            (
                Some(json_type),
                r#"{"name":1,"username":1,"password":1}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                Some(json_type),
                r#"{"name":null,"username":"x","password":null}"#,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
        ];
        for (content_type, body, expected) in cases {
            let reply = app
                .request(
                    Method::POST,
                    path,
                    None,
                    content_type.map(|ct| (ct, body.as_bytes().to_vec())),
                )
                .await;
            assert_eq!(reply.status, expected, "{path} {content_type:?} {body:?}");
        }
    }
}

#[tokio::test]
async fn auth_routes_accept_only_post() {
    let app = test_app().await;
    for path in [
        "/api/auth/admin-login",
        "/api/auth/login",
        "/api/auth/validate-token",
    ] {
        let reply = app.request(Method::GET, path, None, None).await;
        assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
    let reply = app
        .request(Method::POST, "/api/auth/admin-login/", None, None)
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

/// Permission row `/api/auth/*`: anyone, whatever token they carry.
#[tokio::test]
async fn auth_routes_are_public_whatever_the_token() {
    let app = test_app().await;
    seed_juan(&app).await;
    let admin = app.admin_token();
    let other_tenant = app.tenant_token(99, "SOMEONE");
    for token in [
        None,
        Some(admin.as_str()),
        Some(other_tenant.as_str()),
        Some("not-a-jwt"),
    ] {
        let credentials = json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "turnstile_token": FakeCaptcha::VALID_TOKEN });
        let (status, _) = app.post("/api/auth/admin-login", token, credentials).await;
        assert_eq!(status, StatusCode::OK, "admin-login with {token:?}");
        let (status, _) = app
            .post(
                "/api/auth/login",
                token,
                json!({ "name": TENANT_NAME, "turnstile_token": FakeCaptcha::VALID_TOKEN }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "login with {token:?}");
    }
}

// ---- validate-token ----

#[tokio::test]
async fn validate_token_accepts_admin_token_and_rejects_missing_token() {
    let app = test_app().await;
    let (_, body) = admin_login(&app, ADMIN_USERNAME, ADMIN_PASSWORD).await;
    let token = token_of(&body);

    let (status, body) = app
        .send(Method::POST, "/api/auth/validate-token", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["role"], "admin");
    assert_eq!(body["user"]["name"], ADMIN_USERNAME);
    assert_eq!(body["user"]["id"], Value::Null);

    let (status, body) = app
        .send(Method::POST, "/api/auth/validate-token", None, None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "Missing token" }));
}

#[tokio::test]
async fn validate_token_returns_tenant_claims() {
    let app = test_app().await;
    let juan = seed_juan(&app).await;
    let (_, body) = tenant_login(&app, TENANT_NAME).await;
    let token = token_of(&body);
    let exp = claims_of(&app, &token).exp;

    let (status, body) = validate(&app, Some(&format!("Bearer {token}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "user": { "id": juan.id, "name": TENANT_NAME, "role": null, "exp": exp } })
    );

    // Absent claims come back as null.
    let exp = (now() + 60) as usize;
    let bare = app.token(Claims {
        id: None,
        name: None,
        role: None,
        exp,
    });
    let (status, body) = validate(&app, Some(&format!("Bearer {bare}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "user": { "id": null, "name": null, "role": null, "exp": exp } })
    );
}

#[tokio::test]
async fn validate_token_ignores_the_body() {
    let app = test_app().await;
    let token = app.admin_token();
    let reply = app
        .request(
            Method::POST,
            "/api/auth/validate-token",
            Some(&token),
            Some(("text/plain", b"junk".to_vec())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
}

#[tokio::test]
async fn validate_token_distinguishes_missing_and_invalid_tokens() {
    let app = test_app().await;
    let good = app.admin_token();
    let missing = json!({ "error": "Missing token" });
    let invalid = json!({ "error": "Invalid token" });

    for header in [
        format!("bearer {good}"),
        good.clone(),
        "Basic abc".to_string(),
        "Bearer".to_string(),
    ] {
        let (status, body) = validate(&app, Some(&header)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{header}");
        assert_eq!(body, missing, "{header}");
    }

    let other_secret = JwtKeys::new("another-secret")
        .issue(&Claims {
            id: None,
            name: Some(ADMIN_USERNAME.into()),
            role: Some("admin".into()),
            exp: (now() + 3600) as usize,
        })
        .unwrap();
    let expired = app.token(Claims {
        id: Some(1),
        name: Some("ANA".into()),
        role: None,
        exp: (now() - 120) as usize,
    });
    for header in [
        "Bearer not-a-jwt".to_string(),
        format!("Bearer {other_secret}"),
        format!("Bearer {expired}"),
        format!("Bearer Bearer {good}"),
        format!("Bearer  {good}"),
    ] {
        let (status, body) = validate(&app, Some(&header)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{header}");
        assert_eq!(body, invalid, "{header}");
    }
}

// ---- issued tokens on protected routes ----

/// The foundation's extractors accept the tokens accounts issues: a protected
/// route answers 401 "Authentication required" without a valid token, and 403
/// to tenants on admin routes. (Port of the legacy
/// `protected_route_rejects_missing_or_invalid_token`, without depending on
/// another domain's routes.)
#[tokio::test]
async fn issued_tokens_open_protected_routes() {
    let app = test_app().await;
    seed_juan(&app).await;
    let probe = Router::new()
        .route(
            "/any",
            get(|AuthUser(claims): AuthUser| async move { claims.name.unwrap_or_default() }),
        )
        .route(
            "/admin",
            get(|Admin(claims): Admin| async move { claims.name.unwrap_or_default() }),
        )
        .with_state(app.state.clone());
    let call = |path: &'static str, token: Option<String>| {
        let probe = probe.clone();
        async move {
            let mut request = Request::builder().uri(path);
            if let Some(token) = token {
                request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
            }
            let response = probe
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            let status = response.status();
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            (status, String::from_utf8(body.to_vec()).unwrap())
        }
    };

    let admin = token_of(&admin_login(&app, ADMIN_USERNAME, ADMIN_PASSWORD).await.1);
    let tenant = token_of(&tenant_login(&app, TENANT_NAME).await.1);

    assert_eq!(
        call("/admin", Some(admin)).await,
        (StatusCode::OK, ADMIN_USERNAME.to_string())
    );
    assert_eq!(
        call("/any", Some(tenant.clone())).await,
        (StatusCode::OK, TENANT_NAME.to_string())
    );
    assert_eq!(
        call("/admin", Some(tenant)).await,
        (
            StatusCode::FORBIDDEN,
            json!({ "error": "Admin access required" }).to_string()
        )
    );
    let unauthenticated = (
        StatusCode::UNAUTHORIZED,
        json!({ "error": "Authentication required" }).to_string(),
    );
    assert_eq!(call("/any", None).await, unauthenticated);
    assert_eq!(
        call("/any", Some("not-a-jwt".into())).await,
        unauthenticated
    );
}

// ---- repository ----

#[tokio::test]
async fn find_by_name_ignoring_case_matches_whole_names() {
    let app = test_app().await;
    let room = seed_room(app.db(), "Test Room", 1000).await;
    let john = seed_tenant(app.db(), room.id, "John Doe").await;
    seed_tenant(app.db(), room.id, "Jane Doe").await;

    for name in ["John Doe", "john doe", "JOHN DOE"] {
        let found = tenant_read_repo::find_by_name_ignoring_case(app.db(), name)
            .await
            .unwrap();
        assert_eq!(found, vec![john.clone()], "{name:?}");
    }
    for name in ["John Doe ", "John", "John%", ""] {
        let found = tenant_read_repo::find_by_name_ignoring_case(app.db(), name)
            .await
            .unwrap();
        assert!(found.is_empty(), "{name:?}");
    }
}

#[tokio::test]
async fn find_by_name_ignoring_case_reports_database_errors() {
    let app = test_app().await;
    app.db()
        .execute_unprepared("ALTER TABLE tenant RENAME TO tenant_gone")
        .await
        .unwrap();
    assert!(
        tenant_read_repo::find_by_name_ignoring_case(app.db(), "John Doe")
            .await
            .is_err()
    );
}

// ---- login guards: rate limit and captcha ----

/// A login `body` to `path` from the client `ip`; the status, the `Retry-After` header and the JSON body.
async fn login_from(
    app: &TestApp,
    path: &str,
    ip: &str,
    body: Value,
) -> (StatusCode, Option<String>, Value) {
    let reply = app
        .request_with_headers(
            Method::POST,
            path,
            None,
            Some(("application/json", body.to_string().into_bytes())),
            &[("cf-connecting-ip", ip)],
        )
        .await;
    let retry_after = reply
        .headers
        .get(header::RETRY_AFTER)
        .map(|v| v.to_str().unwrap().to_string());
    (reply.status, retry_after, reply.json())
}

#[tokio::test]
async fn logins_are_limited_per_client_and_route() {
    let app = test_app().await;
    seed_juan(&app).await;
    let wrong = json!({ "username": ADMIN_USERNAME, "password": "wrong", "turnstile_token": FakeCaptcha::VALID_TOKEN });
    for _ in 0..LOGIN_LIMIT {
        let (status, _, _) =
            login_from(&app, "/api/auth/admin-login", "203.0.113.1", wrong.clone()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // Over the limit even with the right password.
    let right = json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "turnstile_token": FakeCaptcha::VALID_TOKEN });
    let (status, retry_after, body) =
        login_from(&app, "/api/auth/admin-login", "203.0.113.1", right.clone()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(retry_after.as_deref(), Some("60"));
    assert_eq!(
        body,
        json!({ "error": "Too many attempts. Please wait a minute and try again." })
    );

    // Another client, and the other login, have their own counts.
    let (status, _, _) = login_from(&app, "/api/auth/admin-login", "203.0.113.2", right).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = login_from(
        &app,
        "/api/auth/login",
        "203.0.113.1",
        json!({ "name": TENANT_NAME, "turnstile_token": FakeCaptcha::VALID_TOKEN }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_failing_rate_limiter_lets_logins_through() {
    let app = test_app().await;
    seed_juan(&app).await;
    app.limiter.set_failing(true);
    let (status, body) = tenant_login(&app, TENANT_NAME).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_sent_captcha_token_is_checked() {
    let app = test_app().await;
    seed_juan(&app).await;
    let tenant = |token: &str| json!({ "name": TENANT_NAME, "turnstile_token": token });
    let admin = |token: &str| json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD, "turnstile_token": token });

    for (path, body) in [
        ("/api/auth/login", tenant(FakeCaptcha::VALID_TOKEN)),
        ("/api/auth/admin-login", admin(FakeCaptcha::VALID_TOKEN)),
    ] {
        let (status, body) = app.post(path, None, body).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
    }
    for (path, body) in [
        ("/api/auth/login", tenant("forged")),
        ("/api/auth/admin-login", admin("forged")),
        ("/api/auth/login", tenant("")),
    ] {
        let (status, body) = app.post(path, None, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(
            body,
            json!({ "error": "Verification failed. Please try again." })
        );
    }
    // Checked before the credentials: a forged token with a wrong password is still a 400.
    let (status, _) = app
        .post(
            "/api/auth/admin-login",
            None,
            json!({ "username": "x", "password": "y", "turnstile_token": "forged" }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn logins_go_through_when_the_captcha_cannot_be_checked() {
    // An outage must not lock everyone out; the rate limit still applies.
    let app = test_app().await;
    seed_juan(&app).await;
    app.captcha.set_unreachable(true);
    let (status, body) = app
        .post(
            "/api/auth/login",
            None,
            json!({ "name": TENANT_NAME, "turnstile_token": FakeCaptcha::VALID_TOKEN }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_missing_captcha_token_is_refused() {
    let app = test_app().await;
    seed_juan(&app).await;
    for (path, body) in [
        ("/api/auth/login", json!({ "name": TENANT_NAME })),
        (
            "/api/auth/admin-login",
            json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD }),
        ),
    ] {
        let (status, body) = app.post(path, None, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(
            body,
            json!({ "error": "Verification failed. Please try again." })
        );
    }
}
