use axum::{Router, routing::{get, post, put, delete}};
use crate::handlers::bill_handler;

pub fn bill_routes() -> Router {
    Router::new()
        .route("/", get(bill_handler::get_bills))
        .route("/:tenant_id/bill", get(bill_handler::get_bill_by_tenant))
        .route("/:tenant_id/bills", get(bill_handler::get_bills_by_tenant))
        .route("/", post(bill_handler::create_bill_handler))
        .route("/:id", put(bill_handler::update_bill_handler))
        .route("/:id", delete(bill_handler::delete_bill))
}
