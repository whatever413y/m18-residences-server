use axum::{
    routing::get,
    Router,
};
use crate::handlers::signed_url_handler::{
    get_receipt_signed_url_handler,
    get_payment_signed_url_handler,
};

pub fn signed_url_routes() -> Router {
    Router::new()
        .route(
            "/receipts/:tenant_name/:filename",
            get(get_receipt_signed_url_handler),
        )
        .route(
            "/payments/:filename",
            get(get_payment_signed_url_handler),
        )
}
