use axum::{middleware::from_fn, routing::{delete, get, post, put}, Router};
use crate::{handlers::bill_handler::{
    create_bill_handler, delete_bill, get_bill_by_tenant, get_bills, get_bills_by_tenant, update_bill_handler
}, middleware::jwt::require_admin};

pub fn bill_routes() -> Router {
    Router::new()
        .route("/", get(get_bills))
        .route("/:tenant_id/bill", get(get_bill_by_tenant))
        .route("/:tenant_id/bills", get(get_bills_by_tenant))
        .route("/", post(create_bill_handler)).route_layer(from_fn(require_admin))
        .route("/:id", put(update_bill_handler)).route_layer(from_fn(require_admin))
        .route("/:id", delete(delete_bill)).route_layer(from_fn(require_admin))
}
