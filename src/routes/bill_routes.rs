use axum::{Router, routing::{get, post, put, delete}};
use crate::handlers::bill_handler;

pub fn bill_routes() -> Router {
    Router::new()
        .route("/", get(bill_handler::get_bills))
        .route("/tenant/:id", get(bill_handler::get_bills_by_tenant))
        .route("/:id", get(bill_handler::get_bill))
        // .route("/", post(bill_handler::create_bill))
        // .route("/:id", put(bill_handler::update_bill))
        .route("/:id", delete(bill_handler::delete_bill))
}
