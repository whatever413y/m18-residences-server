use axum::Router;
use axum::routing::get;

use crate::app::AppState;
use crate::property::handlers::tenant_handler::{
    create_tenant, delete_tenant, get_tenant, get_tenant_by_name, get_tenants, update_tenant,
};

pub fn tenant_routes() -> Router<AppState> {
    Router::new()
        .route("/api/tenants", get(get_tenants).post(create_tenant))
        .route(
            "/api/tenants/{id}",
            get(get_tenant).put(update_tenant).delete(delete_tenant),
        )
        .route("/api/tenants/tenant/{name}", get(get_tenant_by_name))
}
