//! API end-to-end tests: requests go through the real router (`app()`) in
//! memory (no socket), backed by the `m18_test` database. Every test starts by
//! truncating all tables; `.cargo/config.toml` makes the tests run one at a time.

mod common;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use m18_residences_server::{app::app, services::r2_service::R2Config};
use serde_json::{Value, json};
use std::path::PathBuf;
use tower::ServiceExt;

const TABLES: [&str; 5] = [
    "additional_charge",
    "bill",
    "electricity_reading",
    "tenant",
    "room",
];

// Synthetic data for the billing flow (also used for the exported fixtures).
const ROOM_NAME: &str = "Room 101";
const ROOM_RENT: i32 = 5000;
const TENANT_NAME: &str = "Juan Dela Cruz";
const JOIN_DATE: &str = "2026-01-01T00:00:00";

// JSON keys the Flutter apps (M18-Residences, M18-Residences-Admin) read.
const ROOM_KEYS: &[&str] = &["id", "name", "rent"];
const TENANT_KEYS: &[&str] = &["id", "room_id", "name", "join_date", "is_active"];
const READING_KEYS: &[&str] = &[
    "id",
    "tenant_id",
    "room_id",
    "prev_reading",
    "curr_reading",
    "consumption",
    "created_at",
];
const BILL_KEYS: &[&str] = &[
    "id",
    "reading_id",
    "tenant_id",
    "room_charges",
    "electric_charges",
    "total_amount",
    "created_at",
    "paid",
    "receipt_url",
];
const CHARGE_KEYS: &[&str] = &["amount", "description"];

// ---------------------- helpers ----------------------

/// Fresh router over an empty test database.
async fn test_app() -> Router {
    // Loads .env.test into the process env. The router must be built after
    // this because cors_layer() reads LOCALHOST_URL at build time.
    let db = common::get_test_db().await;
    for table in TABLES {
        common::reset_table(&db, table).await;
    }
    app(db, dummy_r2())
}

/// R2 client pointing at a fake endpoint: presigning is offline, so the
/// signed-URL endpoints work without network access or real credentials.
fn dummy_r2() -> R2Config {
    let config = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("auto"))
        .credentials_provider(Credentials::new("test", "test", None, None, "test"))
        .endpoint_url("https://example.invalid")
        .build();

    R2Config {
        client: aws_sdk_s3::Client::from_conf(config),
        bucket: "test-bucket".to_string(),
    }
}

/// Sends one request through the router and returns the status and the JSON
/// body (`Value::Null` when the body is empty).
async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let body = body.map(|json| ("application/json".to_string(), json.to_string()));
    send_raw(app, method, uri, token, body).await
}

/// Like `send`, with a raw `(content type, body)` instead of JSON.
async fn send_raw(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<(String, String)>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request_body = match body {
        Some((content_type, body)) => {
            request = request.header(header::CONTENT_TYPE, content_type);
            Body::from(body)
        }
        None => Body::empty(),
    };

    let response = app
        .clone()
        .oneshot(request.body(request_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    if bytes.is_empty() {
        return (status, Value::Null);
    }
    let json = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        panic!(
            "{uri}: expected a JSON body ({err}), got {status}: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, json)
}

async fn get(app: &Router, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
    send(app, Method::GET, uri, token, None).await
}

async fn post(app: &Router, uri: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    send(app, Method::POST, uri, token, Some(body)).await
}

async fn put(app: &Router, uri: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
    send(app, Method::PUT, uri, token, Some(body)).await
}

/// `multipart/form-data` content type and body with text fields only.
fn multipart_form(fields: &[(&str, String)]) -> (String, String) {
    const BOUNDARY: &str = "m18-test-boundary";
    let mut body = String::new();
    for (name, value) in fields {
        body.push_str(&format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        ));
    }
    body.push_str(&format!("--{BOUNDARY}--\r\n"));
    (format!("multipart/form-data; boundary={BOUNDARY}"), body)
}

/// CORS preflight for `GET /api/rooms` from `origin`; returns status and headers.
async fn preflight(app: &Router, origin: &str) -> (StatusCode, HeaderMap) {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/rooms")
        .header(header::ORIGIN, origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    (response.status(), response.headers().clone())
}

/// Admin credentials from the env (.env.test locally, workflow env in CI).
fn admin_credentials() -> (String, String) {
    let username = std::env::var("ADMIN_USERNAME")
        .expect("ADMIN_USERNAME must be set in .env.test (see .env.test.example)");
    let password = std::env::var("ADMIN_PASSWORD")
        .expect("ADMIN_PASSWORD must be set in .env.test (see .env.test.example)");
    assert!(
        !username.is_empty() && !password.is_empty(),
        "ADMIN_USERNAME and ADMIN_PASSWORD must not be empty"
    );
    (username, password)
}

/// Logs in as admin; returns the response body `{token, role, username}`.
async fn admin_login(app: &Router) -> Value {
    let (username, password) = admin_credentials();
    let (status, body) = post(
        app,
        "/api/auth/admin-login",
        None,
        json!({ "username": username, "password": password }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin login: {body}");
    body
}

fn token_of(body: &Value) -> String {
    body["token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .unwrap_or_else(|| panic!("missing token in {body}"))
        .to_string()
}

fn id_of(value: &Value) -> i64 {
    value["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("missing numeric id in {value}"))
}

/// Asserts `value` is a JSON object containing every key in `keys`.
fn assert_keys(value: &Value, keys: &[&str], what: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what}: expected a JSON object, got {value}"));
    for key in keys {
        assert!(
            object.contains_key(*key),
            "{what}: missing key `{key}` in {value}"
        );
    }
}

/// Asserts the `{bill, additional_charges, reading}` shape the Flutter apps parse.
fn assert_bill_details(value: &Value, what: &str) {
    assert_keys(value, &["bill", "additional_charges", "reading"], what);
    assert_keys(&value["bill"], BILL_KEYS, &format!("{what}.bill"));
    let charges = value["additional_charges"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}.additional_charges: expected an array in {value}"));
    for charge in charges {
        assert_keys(charge, CHARGE_KEYS, &format!("{what}.additional_charges[]"));
    }
    assert_keys(&value["reading"], READING_KEYS, &format!("{what}.reading"));
}

async fn create_room(app: &Router, token: &str) -> Value {
    let (status, room) = post(
        app,
        "/api/rooms",
        Some(token),
        json!({ "name": ROOM_NAME, "rent": ROOM_RENT }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create room: {room}");
    assert_keys(&room, ROOM_KEYS, "room");
    room
}

async fn create_tenant(app: &Router, token: &str, room_id: i64) -> Value {
    let (status, tenant) = post(
        app,
        "/api/tenants",
        Some(token),
        json!({ "name": TENANT_NAME, "room_id": room_id, "join_date": JOIN_DATE }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create tenant: {tenant}");
    assert_keys(&tenant, TENANT_KEYS, "tenant");
    tenant
}

/// Everything the admin billing flow produced, as returned by the API.
struct BillingFlow {
    room: Value,
    tenant: Value,
    reading: Value,
    /// `POST /api/bills` response.
    created_bill: Value,
    /// `GET /api/bills`.
    bills: Value,
    /// `GET /api/bills/{tenant_id}/bill`.
    latest_bill: Value,
    /// `GET /api/bills/{tenant_id}/bills`.
    tenant_bills: Value,
}

/// Admin flow: room -> tenant -> reading -> bill, then the three bill reads.
/// Checks status codes and the JSON shapes the Flutter apps parse.
async fn run_billing_flow(app: &Router, token: &str) -> BillingFlow {
    let room = create_room(app, token).await;
    let tenant = create_tenant(app, token, id_of(&room)).await;

    let (status, reading) = post(
        app,
        "/api/electricity-readings",
        Some(token),
        json!({
            "tenant_id": tenant["id"],
            "room_id": room["id"],
            "prev_reading": 100,
            "curr_reading": 150,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create reading: {reading}");
    assert_keys(&reading, READING_KEYS, "reading");

    let (status, created_bill) = post(
        app,
        "/api/bills",
        Some(token),
        json!({
            "tenant_id": tenant["id"],
            "reading_id": reading["id"],
            "room_charges": 5000,
            "electric_charges": 850,
            "additional_charges": [{ "amount": 200, "description": "Water" }],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create bill: {created_bill}");
    assert_bill_details(&created_bill, "created bill");

    let tenant_id = id_of(&tenant);

    let (status, bills) = get(app, "/api/bills", Some(token)).await;
    assert_eq!(status, StatusCode::OK, "list bills: {bills}");
    for bill in bills.as_array().expect("GET /api/bills returns an array") {
        assert_bill_details(bill, "bills[]");
    }

    let (status, latest_bill) =
        get(app, &format!("/api/bills/{tenant_id}/bill"), Some(token)).await;
    assert_eq!(status, StatusCode::OK, "latest bill: {latest_bill}");
    assert_bill_details(&latest_bill, "latest bill");

    let (status, tenant_bills) =
        get(app, &format!("/api/bills/{tenant_id}/bills"), Some(token)).await;
    assert_eq!(status, StatusCode::OK, "tenant bills: {tenant_bills}");
    for bill in tenant_bills
        .as_array()
        .expect("GET /api/bills/{id}/bills returns an array")
    {
        assert_bill_details(bill, "tenant bills[]");
    }

    BillingFlow {
        room,
        tenant,
        reading,
        created_bill,
        bills,
        latest_bill,
        tenant_bills,
    }
}

/// Replaces every JWT with "<token>" and every presigned URL with "<signed-url>".
fn redact(value: &mut Value) {
    match value {
        Value::String(s) if is_jwt(s) => *s = "<token>".to_string(),
        Value::String(s) if s.contains("X-Amz-Signature=") => *s = "<signed-url>".to_string(),
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::Object(map) => map.values_mut().for_each(redact),
        _ => {}
    }
}

fn is_jwt(s: &str) -> bool {
    let segments: Vec<&str> = s.split('.').collect();
    s.starts_with("eyJ")
        && segments.len() == 3
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

// ---------------------- tests ----------------------

#[tokio::test]
async fn health_returns_ok() {
    let app = test_app().await;

    let (status, body) = get(&app, "/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn cors_preflight_allows_only_configured_origins() {
    let app = test_app().await;

    // LOCALHOST_URL in .env.test / CI lists the admin (50001) and tenant (50002) apps.
    for origin in ["http://localhost:50001", "http://localhost:50002"] {
        let (status, headers) = preflight(&app, origin).await;
        assert_eq!(status, StatusCode::OK, "preflight from {origin}");
        assert_eq!(
            headers
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|value| value.to_str().unwrap()),
            Some(origin),
            "preflight from {origin}: {headers:?}"
        );
    }

    let (_, headers) = preflight(&app, "http://localhost:50003").await;
    assert!(
        headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none(),
        "unlisted origin must not be allowed: {headers:?}"
    );
}

#[tokio::test]
async fn admin_login_returns_token_and_rejects_wrong_password() {
    let app = test_app().await;
    let (username, password) = admin_credentials();

    let body = admin_login(&app).await;
    assert!(!token_of(&body).is_empty());
    assert_eq!(body["role"], "admin");
    assert_eq!(body["username"], username.as_str());

    let (status, body) = post(
        &app,
        "/api/auth/admin-login",
        None,
        json!({ "username": username, "password": format!("{password}-wrong") }),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "Invalid credentials" }));
}

#[tokio::test]
async fn protected_route_rejects_missing_or_invalid_token() {
    let app = test_app().await;
    let expected = json!({ "error": "Authentication required" });

    let (status, body) = get(&app, "/api/rooms", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, expected);

    let (status, body) = get(&app, "/api/rooms", Some("not-a-jwt")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, expected);
}

#[tokio::test]
async fn validate_token_accepts_admin_token_and_rejects_missing_token() {
    let app = test_app().await;
    let (username, _) = admin_credentials();
    let token = token_of(&admin_login(&app).await);

    let (status, body) = send(
        &app,
        Method::POST,
        "/api/auth/validate-token",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["role"], "admin");
    assert_eq!(body["user"]["name"], username.as_str());

    let (status, body) = send(&app, Method::POST, "/api/auth/validate-token", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({ "error": "Missing token" }));
}

#[tokio::test]
async fn admin_billing_flow_returns_contract_shapes() {
    let app = test_app().await;
    let token = token_of(&admin_login(&app).await);

    let flow = run_billing_flow(&app, &token).await;

    assert_eq!(flow.room["name"], ROOM_NAME);
    assert_eq!(flow.room["rent"], ROOM_RENT);

    assert_eq!(flow.tenant["room_id"], flow.room["id"]);
    assert_eq!(flow.tenant["name"], TENANT_NAME);
    assert_eq!(flow.tenant["join_date"], JOIN_DATE);
    assert_eq!(flow.tenant["is_active"], true);

    assert_eq!(flow.reading["tenant_id"], flow.tenant["id"]);
    assert_eq!(flow.reading["room_id"], flow.room["id"]);
    assert_eq!(flow.reading["prev_reading"], 100);
    assert_eq!(flow.reading["curr_reading"], 150);
    assert_eq!(
        flow.reading["consumption"], 50,
        "consumption is computed by the server"
    );

    let bill = &flow.created_bill["bill"];
    assert_eq!(bill["tenant_id"], flow.tenant["id"]);
    assert_eq!(bill["reading_id"], flow.reading["id"]);
    assert_eq!(bill["room_charges"], 5000);
    assert_eq!(bill["electric_charges"], 850);
    assert_eq!(
        bill["total_amount"], 6050,
        "total = room + electric + additional, computed by the server"
    );
    assert_eq!(bill["paid"], false);
    assert_eq!(bill["receipt_url"], Value::Null);
    assert_eq!(flow.created_bill["reading"], flow.reading);
    let charges = flow.created_bill["additional_charges"].as_array().unwrap();
    assert_eq!(charges.len(), 1);
    assert_eq!(charges[0]["amount"], 200);
    assert_eq!(charges[0]["description"], "Water");

    // The read endpoints return the same bill, in the same shape.
    assert_eq!(flow.bills, json!([flow.created_bill]));
    assert_eq!(flow.latest_bill, flow.created_bill);
    assert_eq!(flow.tenant_bills, json!([flow.created_bill]));
}

#[tokio::test]
async fn tenant_login_returns_tenant_and_token_reads_own_bill() {
    let app = test_app().await;
    let admin_token = token_of(&admin_login(&app).await);
    let flow = run_billing_flow(&app, &admin_token).await;

    let (status, body) = post(
        &app,
        "/api/auth/login",
        None,
        json!({ "name": TENANT_NAME }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "tenant login: {body}");
    let tenant_token = token_of(&body);
    assert_keys(&body["tenant"], TENANT_KEYS, "tenant login.tenant");
    assert_eq!(body["tenant"], flow.tenant);

    let (status, body) = post(
        &app,
        "/api/auth/login",
        None,
        json!({ "name": "Unknown Tenant" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, json!({ "error": "Tenant not found" }));

    let tenant_id = id_of(&flow.tenant);
    let (status, bill) = get(
        &app,
        &format!("/api/bills/{tenant_id}/bill"),
        Some(&tenant_token),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "latest bill with tenant token: {bill}"
    );
    assert_eq!(bill, flow.created_bill);
}

#[tokio::test]
async fn latest_bill_is_404_for_tenant_without_bills() {
    let app = test_app().await;
    let token = token_of(&admin_login(&app).await);
    let room = create_room(&app, &token).await;
    let tenant = create_tenant(&app, &token, id_of(&room)).await;

    let tenant_id = id_of(&tenant);
    let (status, body) = get(&app, &format!("/api/bills/{tenant_id}/bill"), Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, Value::Null, "the handler returns a bare 404");
}

/// Every route with path parameters resolves to its handler (a route with a
/// bad pattern would only surface as a 404) and returns what the admin app expects.
#[tokio::test]
async fn parameterized_routes_resolve_and_update_or_delete() {
    let app = test_app().await;
    let admin_token = token_of(&admin_login(&app).await);
    let token = Some(admin_token.as_str());
    let flow = run_billing_flow(&app, &admin_token).await;
    let room_id = id_of(&flow.room);
    let tenant_id = id_of(&flow.tenant);
    let reading_id = id_of(&flow.reading);
    let bill_id = id_of(&flow.created_bill["bill"]);

    let tenant_by_name = format!("/api/tenants/tenant/{}", TENANT_NAME.replace(' ', "%20"));
    for (uri, expected) in [
        (format!("/api/rooms/{room_id}"), &flow.room),
        (format!("/api/tenants/{tenant_id}"), &flow.tenant),
        (tenant_by_name, &flow.tenant),
        (
            format!("/api/electricity-readings/{reading_id}"),
            &flow.reading,
        ),
    ] {
        let (status, body) = get(&app, &uri, token).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}: {body}");
        assert_eq!(&body, expected, "GET {uri}");
    }

    let (status, room) = put(
        &app,
        &format!("/api/rooms/{room_id}"),
        token,
        json!({ "name": "Room 102", "rent": 5500 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update room: {room}");
    assert_eq!(
        (&room["name"], &room["rent"]),
        (&json!("Room 102"), &json!(5500))
    );

    let (status, tenant) = put(
        &app,
        &format!("/api/tenants/{tenant_id}"),
        token,
        json!({ "name": TENANT_NAME, "room_id": room_id, "join_date": JOIN_DATE, "is_active": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update tenant: {tenant}");
    assert_eq!(tenant["is_active"], false);

    let (status, reading) = put(
        &app,
        &format!("/api/electricity-readings/{reading_id}"),
        token,
        json!({ "tenant_id": tenant_id, "room_id": room_id, "prev_reading": 100, "curr_reading": 170 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update reading: {reading}");
    assert_eq!(reading["consumption"], 70);

    let (status, bill) = put(
        &app,
        &format!("/api/bills/{bill_id}"),
        token,
        json!({
            "tenant_id": tenant_id,
            "reading_id": reading_id,
            "room_charges": 5000,
            "electric_charges": 1190,
            "additional_charges": [{ "amount": 300, "description": "Internet" }],
            "receipt_url": null,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update bill: {bill}");
    assert_bill_details(&bill, "updated bill");
    assert_eq!(bill["bill"]["total_amount"], 6490);

    // Multipart update without a file part, so nothing is uploaded to R2.
    let form = multipart_form(&[
        ("tenant_id", tenant_id.to_string()),
        ("reading_id", reading_id.to_string()),
        ("room_charges", "5000".to_string()),
        ("electric_charges", "1190".to_string()),
        (
            "additional_charges",
            json!([{ "amount": 100, "description": "Water" }]).to_string(),
        ),
    ]);
    let (status, bill) = send_raw(
        &app,
        Method::PUT,
        &format!("/api/bills/{bill_id}/upload"),
        token,
        Some(form),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "multipart bill update: {bill}");
    assert_eq!(bill["bill"]["total_amount"], 6290);

    let (status, body) = get(&app, "/api/signed-urls/receipts/juan/1700000000-r1", token).await;
    assert_eq!(status, StatusCode::OK, "receipt signed url: {body}");
    let url = body["url"].as_str().unwrap_or_default();
    assert!(url.contains("receipts/juan/1700000000-r1"), "{body}");

    // Children first: the foreign keys restrict deletes.
    for uri in [
        format!("/api/bills/{bill_id}"),
        format!("/api/electricity-readings/{reading_id}"),
        format!("/api/tenants/{tenant_id}"),
        format!("/api/rooms/{room_id}"),
    ] {
        let (status, body) = send(&app, Method::DELETE, &uri, token, None).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "DELETE {uri}: {body}");
    }
    let (status, rooms) = get(&app, "/api/rooms", token).await;
    assert_eq!((status, rooms), (StatusCode::OK, json!([])));
}

#[tokio::test]
async fn payment_signed_url_is_presigned_offline() {
    let app = test_app().await;
    let token = token_of(&admin_login(&app).await);

    let (status, body) = get(&app, "/api/signed-urls/payments/gcash", Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let url = body["url"]
        .as_str()
        .unwrap_or_else(|| panic!("missing url in {body}"));
    assert!(url.contains("payments/gcash.png"), "{url}");
    assert!(
        url.contains("X-Amz-Signature="),
        "expected a presigned URL: {url}"
    );
}

/// Writes the API responses the Flutter apps consume as JSON fixtures
/// (relative FIXTURES_OUT paths resolve from the repository root):
/// `$env:FIXTURES_OUT='<dir>'; cargo test export_contract_fixtures -- --ignored`
#[tokio::test]
#[ignore = "writes contract fixtures; needs FIXTURES_OUT and --ignored"]
async fn export_contract_fixtures() {
    let out_dir = std::env::var_os("FIXTURES_OUT")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            panic!(
                "FIXTURES_OUT is not set: set it to the directory the contract fixtures \
                 are written to, e.g. $env:FIXTURES_OUT='C:\\path\\to\\fixtures'; \
                 cargo test export_contract_fixtures -- --ignored"
            )
        });

    let app = test_app().await;
    let admin = admin_login(&app).await;
    let admin_token = token_of(&admin);
    let flow = run_billing_flow(&app, &admin_token).await;

    let (status, tenant_login) = post(
        &app,
        "/api/auth/login",
        None,
        json!({ "name": TENANT_NAME }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "tenant login: {tenant_login}");

    let (status, signed_url) =
        get(&app, "/api/signed-urls/payments/gcash", Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK, "signed url: {signed_url}");

    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|err| panic!("creating {}: {err}", out_dir.display()));

    let fixtures = [
        ("admin_login.json", admin),
        ("tenant_login.json", tenant_login),
        ("room.json", flow.room),
        ("tenant.json", flow.tenant),
        ("reading.json", flow.reading),
        ("bill.json", flow.created_bill),
        ("bills.json", flow.bills),
        ("latest_bill.json", flow.latest_bill),
        ("signed_url.json", signed_url),
    ];
    for (name, mut value) in fixtures {
        redact(&mut value);
        let text = serde_json::to_string_pretty(&value).unwrap();
        assert!(
            !text.contains("eyJ") && !text.contains("X-Amz-"),
            "{name}: token or signed URL left after redaction: {text}"
        );

        let path = out_dir.join(name);
        std::fs::write(&path, text + "\n")
            .unwrap_or_else(|err| panic!("writing {}: {err}", path.display()));
        println!("wrote {}", path.display());
    }
}
