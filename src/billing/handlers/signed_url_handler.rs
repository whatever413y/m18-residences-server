use axum::{
    Json,
    extract::State,
    http::{HeaderMap, Uri, header::HOST},
};
use m18_residences_shared_rs::{auth::AuthUser, error::ApiError, extract::ValidPath};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::services::signed_url_service::{self, SignedUrl, check_segment},
};

#[derive(Deserialize)]
pub struct ReceiptPath {
    pub tenant_name: String,
    pub filename: String,
}

#[derive(Deserialize)]
pub struct PaymentPath {
    pub name: String,
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

/// GET /api/signed-urls/receipts/{tenant_name}/{filename} (admin, or that tenant)
pub async fn get_receipt_signed_url_handler(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    uri: Uri,
    headers: HeaderMap,
    path: ValidPath<ReceiptPath>,
) -> Result<Json<SignedUrl>, ApiError> {
    let ValidPath(ReceiptPath {
        tenant_name,
        filename,
    }) = path;
    check_segment("tenant_name", &tenant_name)?;
    check_segment("filename", &filename)?;
    claims.ensure_admin_or_name(&tenant_name)?;
    let origin = request_origin(&uri, &headers)?;
    Ok(Json(
        signed_url_service::receipt_link(
            state.files.as_ref(),
            &state.signer,
            &origin,
            &tenant_name,
            &filename,
        )
        .await?,
    ))
}

/// GET /api/signed-urls/payments/{name} (any logged-in user)
pub async fn get_payment_signed_url_handler(
    State(state): State<AppState>,
    _user: AuthUser,
    uri: Uri,
    headers: HeaderMap,
    path: ValidPath<PaymentPath>,
) -> Result<Json<SignedUrl>, ApiError> {
    let ValidPath(PaymentPath { name }) = path;
    check_segment("name", &name)?;
    let origin = request_origin(&uri, &headers)?;
    Ok(Json(
        signed_url_service::payment_link(state.files.as_ref(), &state.signer, &origin, &name)
            .await?,
    ))
}
