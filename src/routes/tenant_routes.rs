use crate::handlers::tenant_handler::{
    create_tenant, delete_tenant, get_tenant, get_tenant_by_name, get_tenants, update_tenant,
};
use axum::Router;
use axum::routing::{delete, get, post, put};

pub fn tenant_routes() -> Router {
    Router::new()
        .route("/", get(get_tenants))
        .route("/:id", get(get_tenant))
        .route("/tenant/:name", get(get_tenant_by_name))
        .route("/", post(create_tenant))
        .route("/:id", put(update_tenant))
        .route("/:id", delete(delete_tenant))
}
