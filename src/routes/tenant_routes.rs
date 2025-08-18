use axum::middleware::from_fn;
use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::tenant_handler::{
    get_tenants, get_tenant, get_tenant_by_name, create_tenant, update_tenant, delete_tenant
};
use crate::middleware::jwt::require_admin;

pub fn tenant_routes() -> Router {
    Router::new()
        .route("/", get(get_tenants))
        .route("/:id", get(get_tenant))
        .route("/tenant/:name", get(get_tenant_by_name))
        .route("/", post(create_tenant)).route_layer(from_fn(require_admin))
        .route("/:id", put(update_tenant)).route_layer(from_fn(require_admin))
        .route("/:id", delete(delete_tenant)).route_layer(from_fn(require_admin))
}
