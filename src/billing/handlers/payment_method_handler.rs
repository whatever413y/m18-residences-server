use axum::{
    Json,
    extract::{
        State,
        multipart::{Multipart, MultipartError, MultipartRejection},
    },
    http::StatusCode,
};
use m18_residences_shared_rs::{
    auth::{Admin, AuthUser},
    error::ApiError,
    extract::{ValidJson, ValidPath},
};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::services::payment_method_service::{self, PaymentMethodInput, PaymentMethodOut},
};

/// The largest multipart body a QR image upload accepts.
pub const PAYMENT_UPLOAD_LIMIT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
pub struct PaymentMethodId {
    pub id: i32,
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

/// The bytes of the multipart `file` part (other parts are ignored; 400 without one).
pub async fn read_file_part(
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Vec<u8>, ApiError> {
    let mut multipart =
        multipart.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let mut file = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() == Some("file") {
            file = Some(field.bytes().await.map_err(multipart_error)?.to_vec());
        }
    }
    file.ok_or_else(|| ApiError::BadRequest("file is required".into()))
}

/// GET /api/payment-methods (any logged-in user), in list order.
pub async fn list_payment_methods(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<PaymentMethodOut>>, ApiError> {
    let methods = payment_method_service::list(&state.db).await?;
    Ok(Json(methods.into_iter().map(Into::into).collect()))
}

/// POST /api/payment-methods (admin)
pub async fn create_payment_method(
    State(state): State<AppState>,
    _admin: Admin,
    ValidJson(input): ValidJson<PaymentMethodInput>,
) -> Result<(StatusCode, Json<PaymentMethodOut>), ApiError> {
    let created = payment_method_service::create(&state.db, input).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// PUT /api/payment-methods/{id} (admin): replaces the name and account details (and the place, when given).
pub async fn update_payment_method(
    State(state): State<AppState>,
    _admin: Admin,
    ValidPath(PaymentMethodId { id }): ValidPath<PaymentMethodId>,
    ValidJson(input): ValidJson<PaymentMethodInput>,
) -> Result<Json<PaymentMethodOut>, ApiError> {
    let updated = payment_method_service::update(&state.db, id, input).await?;
    Ok(Json(updated.into()))
}

/// DELETE /api/payment-methods/{id} (admin): its QR image is archived.
pub async fn delete_payment_method(
    State(state): State<AppState>,
    _admin: Admin,
    ValidPath(PaymentMethodId { id }): ValidPath<PaymentMethodId>,
) -> Result<StatusCode, ApiError> {
    payment_method_service::delete(&state.db, state.files.as_ref(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/payment-methods/{id}/image (admin; multipart with the PNG in the `file` part): the old image is archived.
pub async fn upload_payment_method_image(
    State(state): State<AppState>,
    _admin: Admin,
    ValidPath(PaymentMethodId { id }): ValidPath<PaymentMethodId>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<PaymentMethodOut>, ApiError> {
    let bytes = read_file_part(multipart).await?;
    let updated =
        payment_method_service::replace_image(&state.db, state.files.as_ref(), id, bytes).await?;
    Ok(Json(updated.into()))
}

/// DELETE /api/payment-methods/{id}/image (admin): the image is archived.
pub async fn remove_payment_method_image(
    State(state): State<AppState>,
    _admin: Admin,
    ValidPath(PaymentMethodId { id }): ValidPath<PaymentMethodId>,
) -> Result<Json<PaymentMethodOut>, ApiError> {
    let updated = payment_method_service::remove_image(&state.db, state.files.as_ref(), id).await?;
    Ok(Json(updated.into()))
}
