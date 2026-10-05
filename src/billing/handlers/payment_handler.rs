use axum::{
    Json,
    extract::{
        State,
        multipart::{Multipart, MultipartError, MultipartRejection},
    },
    http::StatusCode,
};
use m18_residences_shared_rs::{auth::Admin, error::ApiError, extract::ValidPath};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::services::payment_service::{self, PaymentImage},
};

/// The largest multipart body `PUT /api/payments/{name}` accepts.
pub const PAYMENT_UPLOAD_LIMIT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
pub struct PaymentName {
    pub name: String,
}

fn multipart_error(err: MultipartError) -> ApiError {
    if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge(format!(
            "The upload is larger than {} MiB",
            PAYMENT_UPLOAD_LIMIT_BYTES / (1024 * 1024)
        ))
    } else {
        ApiError::BadRequest(err.body_text())
    }
}

/// GET /api/payments (admin)
pub async fn list_payments(
    State(state): State<AppState>,
    _admin: Admin,
) -> Result<Json<Vec<PaymentImage>>, ApiError> {
    Ok(Json(payment_service::list(state.files.as_ref()).await?))
}

/// PUT /api/payments/{name} (admin; multipart with the PNG in the `file` part)
pub async fn replace_payment(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<PaymentName>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<PaymentImage>, ApiError> {
    let ValidPath(PaymentName { name }) = path;
    let mut multipart =
        multipart.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let mut file = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() == Some("file") {
            file = Some(field.bytes().await.map_err(multipart_error)?.to_vec());
        }
        // Other parts are ignored.
    }
    let bytes = file.ok_or_else(|| ApiError::BadRequest("file is required".into()))?;
    Ok(Json(
        payment_service::replace(state.files.as_ref(), &name, bytes).await?,
    ))
}
