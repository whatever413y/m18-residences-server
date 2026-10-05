//! Routes and layers outside the domains: health checks and CORS.
mod common;

use axum::http::{StatusCode, header};
use common::{ADMIN_ORIGIN, TENANT_ORIGIN, test_app};
use serde_json::json;

#[tokio::test]
async fn health_reports_ok_and_the_version() {
    let app = test_app().await;
    let (status, body) = app.get("/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok", "version": "test-version" }));

    let reply = app.request(axum::http::Method::GET, "/", None, None).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, b"API is up");
}

#[tokio::test]
async fn cors_preflight_allows_only_configured_origins() {
    let app = test_app().await;
    // The last one matches the pattern https://*-admin.preview.test (preview links).
    for origin in [
        ADMIN_ORIGIN,
        TENANT_ORIGIN,
        "https://pr-3-admin.preview.test",
    ] {
        let reply = app.preflight("/api/rooms", origin).await;
        assert_eq!(reply.status, StatusCode::OK, "{origin}");
        assert_eq!(
            reply
                .headers
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            origin
        );
        assert_eq!(
            reply
                .headers
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .unwrap(),
            "true"
        );
    }
    for origin in ["https://evil.example", "https://a.b-admin.preview.test"] {
        let reply = app.preflight("/api/rooms", origin).await;
        assert!(
            reply
                .headers
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none(),
            "{origin}"
        );
    }
}

#[tokio::test]
async fn unknown_routes_are_404() {
    let app = test_app().await;
    let reply = app
        .request(axum::http::Method::GET, "/api/nope", None, None)
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}
