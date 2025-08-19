use axum::{extract::DefaultBodyLimit, routing::{delete, get, post, put}, Router};
use crate::{handlers::bill_handler::{
    create_bill_handler, delete_bill, get_bill_by_tenant, get_bills, get_bills_by_tenant, update_bill_json_handler, update_bill_multipart_handler
}};

pub fn bill_routes() -> Router {
    Router::new()
        .route("/", get(get_bills))
        .route("/:tenant_id/bill", get(get_bill_by_tenant))
        .route("/:tenant_id/bills", get(get_bills_by_tenant))
        .route("/", post(create_bill_handler)) 
        .route("/:id", put(update_bill_json_handler))
        .route("/:id/upload", put(update_bill_multipart_handler).route_layer(DefaultBodyLimit::max(10485760)))
        .route("/:id", delete(delete_bill)) 
}
