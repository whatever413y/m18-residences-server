use axum::{middleware::from_fn, response::Json, routing::get, Extension, Router};
use sea_orm::DatabaseConnection;

use crate::middleware::{cors::cors_layer, jwt::require_auth};
use crate::routes;
use crate::services::r2_service::R2Config;

/// Builds the application router: public routes, JWT-protected API routes and
/// the global layers (CORS, DB connection, R2 client).
///
/// `cors_layer()` reads `LOCALHOST_URL`/`PRODUCTION_URL` when this runs, so the
/// environment must be loaded before calling it.
pub fn app(db: DatabaseConnection, r2: R2Config) -> Router {
    // Helper to apply JWT auth to a router
    let protected = |router: Router| router.route_layer(from_fn(require_auth));

    Router::new()
        // Public routes
        .nest("/api/auth", routes::auth_routes::auth_routes())
        .route("/", get(|| async { "API is up" }))
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))

        // Protected routes
        .nest("/api/signed-urls", protected(routes::signed_url_routes::signed_url_routes()))
        .nest("/api/rooms", protected(routes::room_routes::room_routes()))
        .nest("/api/tenants", protected(routes::tenant_routes::tenant_routes()))
        .nest(
            "/api/electricity-readings",
            protected(routes::electricity_reading_routes::electricity_reading_routes()),
        )
        .nest("/api/bills", protected(routes::bill_routes::bill_routes()))

        // Global layers
        .layer(cors_layer())
        .layer(Extension(db))
        .layer(Extension(r2))
}
