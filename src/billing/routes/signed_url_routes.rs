use axum::{Router, middleware::from_extractor_with_state, routing::get};
use m18_residences_shared_rs::auth::AuthUser;

use crate::{
    app::AppState,
    billing::handlers::signed_url_handler::{
        get_payment_method_signed_url_handler, get_payment_signed_url_handler,
        get_receipt_signed_url_handler, get_tenant_payment_signed_url_handler,
    },
};

/// `/api/signed-urls`: logged-in users only; receipts and tenant payments are checked per tenant.
pub fn signed_url_routes(state: &AppState) -> Router<AppState> {
    let signed_urls = Router::new()
        .route(
            "/receipts/{tenant_name}/{filename}",
            get(get_receipt_signed_url_handler),
        )
        .route(
            "/tenant-payments/{tenant_name}/{filename}",
            get(get_tenant_payment_signed_url_handler),
        )
        .route(
            "/payment-methods/{id}",
            get(get_payment_method_signed_url_handler),
        )
        .route("/payments/{name}", get(get_payment_signed_url_handler))
        .route_layer(from_extractor_with_state::<AuthUser, AppState>(
            state.clone(),
        ));
    Router::new().nest("/api/signed-urls", signed_urls)
}
