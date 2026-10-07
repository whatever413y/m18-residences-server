//! Transitional: the payment-image routes the apps used before payment methods (see `payment_service`).
use axum::{
    Json,
    extract::{
        State,
        multipart::{Multipart, MultipartRejection},
    },
};
use m18_residences_shared_rs::{auth::Admin, error::ApiError, extract::ValidPath};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::{
        handlers::payment_method_handler::read_file_part,
        services::payment_service::{self, PaymentImage},
    },
};

#[derive(Deserialize)]
pub struct PaymentName {
    pub name: String,
}

/// GET /api/payments (admin)
pub async fn list_payments(
    State(state): State<AppState>,
    _admin: Admin,
) -> Result<Json<Vec<PaymentImage>>, ApiError> {
    Ok(Json(
        payment_service::list(&state.db, state.files.as_ref()).await?,
    ))
}

/// PUT /api/payments/{name} (admin; multipart with the PNG in the `file` part)
pub async fn replace_payment(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<PaymentName>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<PaymentImage>, ApiError> {
    let ValidPath(PaymentName { name }) = path;
    let bytes = read_file_part(multipart).await?;
    Ok(Json(
        payment_service::replace(&state.db, state.files.as_ref(), &name, bytes).await?,
    ))
}
