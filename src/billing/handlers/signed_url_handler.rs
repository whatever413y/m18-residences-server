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
        payment_method_service, payment_service,
        signed_url_service::{self, SignedUrl, check_segment},
    },
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

#[derive(Deserialize)]
pub struct PaymentMethodPath {
    pub id: i32,
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

/// GET /api/signed-urls/tenant-payments/{tenant_name}/{filename} (admin, or that tenant)
pub async fn get_tenant_payment_signed_url_handler(
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
        signed_url_service::tenant_payment_link(
            state.files.as_ref(),
            &state.signer,
            &origin,
            &tenant_name,
            &filename,
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

/// GET /api/signed-urls/payments/{name} (any logged-in user). Transitional: the QR image of the method whose name
/// has this slug; remove with the other `/api/payments` routes.
pub async fn get_payment_signed_url_handler(
    State(state): State<AppState>,
    _user: AuthUser,
    uri: Uri,
    headers: HeaderMap,
    path: ValidPath<PaymentPath>,
) -> Result<Json<SignedUrl>, ApiError> {
    let ValidPath(PaymentPath { name }) = path;
    check_segment("name", &name)?;
    let key = payment_service::image_key(&state.db, &name).await?;
    let origin = request_origin(&uri, &headers)?;
    Ok(Json(
        signed_url_service::payment_method_link(state.files.as_ref(), &state.signer, &origin, &key)
            .await?,
    ))
}
