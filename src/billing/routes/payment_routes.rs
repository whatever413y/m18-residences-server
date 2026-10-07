use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware::from_extractor_with_state,
    routing::{get, put},
};
use m18_residences_shared_rs::auth::AuthUser;

use crate::{
    app::AppState,
    billing::handlers::{
        payment_handler::{list_payments, replace_payment},
        payment_method_handler::PAYMENT_UPLOAD_LIMIT_BYTES,
    },
};

/// `/api/payments`: transitional, the payment QR images by method slug, admin only (see `payment_service`).
pub fn payment_routes(state: &AppState) -> Router<AppState> {
    let payments = Router::new()
        .route("/", get(list_payments))
        .route(
            "/{name}",
            put(replace_payment).layer(DefaultBodyLimit::max(PAYMENT_UPLOAD_LIMIT_BYTES)),
        )
        .route_layer(from_extractor_with_state::<AuthUser, AppState>(
            state.clone(),
        ));
    Router::new().nest("/api/payments", payments)
}
