use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware::from_extractor_with_state,
    routing::{get, put},
};
use m18_residences_shared_rs::auth::AuthUser;

use crate::{
    app::AppState,
    billing::handlers::payment_method_handler::{
        PAYMENT_UPLOAD_LIMIT_BYTES, create_payment_method, delete_payment_method,
        list_payment_methods, remove_payment_method_image, update_payment_method,
        upload_payment_method_image,
    },
};

/// `/api/payment-methods`: listed for every logged-in user, changed by the admin only (QR images are read through
/// `/api/signed-urls/payment-methods/{id}`).
pub fn payment_method_routes(state: &AppState) -> Router<AppState> {
    let methods = Router::new()
        .route("/", get(list_payment_methods).post(create_payment_method))
        .route(
            "/{id}",
            put(update_payment_method).delete(delete_payment_method),
        )
        .route(
            "/{id}/image",
            put(upload_payment_method_image)
                .delete(remove_payment_method_image)
                .layer(DefaultBodyLimit::max(PAYMENT_UPLOAD_LIMIT_BYTES)),
        )
        .route_layer(from_extractor_with_state::<AuthUser, AppState>(
            state.clone(),
        ));
    Router::new().nest("/api/payment-methods", methods)
}
