use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::tenant_handler;

pub fn tenant_routes() -> Router {
    Router::new()
        .route("/", get(tenant_handler::get_tenants))
        .route("/:id", get(tenant_handler::get_tenant))
        .route("/tenant/:name", get(tenant_handler::get_tenant_by_name))
        .route("/", post(tenant_handler::create_tenant))
        .route("/:id", put(tenant_handler::update_tenant))
        .route("/:id", delete(tenant_handler::delete_tenant))
}
