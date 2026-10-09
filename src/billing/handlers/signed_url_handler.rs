use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Uri, header::HOST},
};
use m18_residences_shared_rs::{auth::AuthUser, error::ApiError, extract::ValidPath};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::services::{
        bill_service::{self, BillFile},
        payment_method_service,
        signed_url_service::{self, SignedUrl},
    },
};

#[derive(Deserialize)]
pub struct PaymentMethodPath {
    pub id: i32,
}

#[derive(Deserialize)]
pub struct BillFilePath {
    pub id: i32,
    /// `receipt` or `payment`.
    pub kind: BillFile,
}

/// Where the API is reached from: the request URI's scheme and authority when
/// absolute (Workers), else `http://<Host>`.
fn request_origin(uri: &Uri, headers: &HeaderMap) -> Result<String, ApiError> {
    if let (Some(scheme), Some(authority)) = (uri.scheme_str(), uri.authority()) {
        return Ok(format!("{scheme}://{authority}"));
    }
    let host = headers
        .get(HOST)
        .and_then(|h| h.to_str().ok())
        .filter(|h| !h.is_empty())
        .ok_or_else(|| ApiError::BadRequest("Missing Host header".into()))?;
    Ok(format!("http://{host}"))
}

/// GET /api/signed-urls/bills/{id}/{receipt|payment} (admin, or the bill's tenant): the bill's
/// file wherever it is stored, so renaming the tenant or moving the bill doesn't lose it.
pub async fn get_bill_file_signed_url_handler(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    uri: Uri,
    headers: HeaderMap,
    path: ValidPath<BillFilePath>,
) -> Result<Json<SignedUrl>, ApiError> {
    let ValidPath(BillFilePath { id, kind }) = path;
    let key = bill_service::file_key_for(&state.db, &claims, id, kind).await?;
    let origin = request_origin(&uri, &headers)?;
    Ok(Json(
        signed_url_service::bill_file_link(
            state.files.as_ref(),
            &state.signer,
            &origin,
            &key,
            &format!("{} not found", kind.label()),
        )
        .await?,
    ))
}

/// GET /api/signed-urls/payment-methods/{id} (any logged-in user): the method's QR image.
pub async fn get_payment_method_signed_url_handler(
    State(state): State<AppState>,
    _user: AuthUser,
    uri: Uri,
    headers: HeaderMap,
    path: ValidPath<PaymentMethodPath>,
) -> Result<Json<SignedUrl>, ApiError> {
    let ValidPath(PaymentMethodPath { id }) = path;
    let key = payment_method_service::image_key(&state.db, id).await?;
    let origin = request_origin(&uri, &headers)?;
    Ok(Json(
        signed_url_service::payment_method_link(state.files.as_ref(), &state.signer, &origin, &key)
            .await?,
    ))
}
