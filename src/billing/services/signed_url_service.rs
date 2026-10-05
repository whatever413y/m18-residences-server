//! Short-lived signed links to stored files (receipts, tenants' payment images
//! and the payment QR images),
//! served by `GET /api/files/...`.
use chrono::Utc;
use m18_residences_shared_rs::{
    error::ApiError,
    files::{FileSigner, FileStore, LINK_TTL_SECONDS},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SignedUrl {
    pub url: String,
    pub content_type: String,
}

/// Fallback for files stored without a content type.
pub const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

/// A path parameter must be one non-empty segment that cannot climb out of its folder.
pub fn check_segment(field: &str, value: &str) -> Result<(), ApiError> {
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} must not be empty")));
    }
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(ApiError::BadRequest(format!(
            "{field} must not contain '/', '\\' or '..'"
        )));
    }
    Ok(())
}

/// A link to `key`, valid for [`LINK_TTL_SECONDS`]; `not_found` is the 404
/// message when there is no such file.
async fn link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    key: &str,
    not_found: &str,
) -> Result<SignedUrl, ApiError> {
    let info = files
        .head(key)
        .await?
        .ok_or_else(|| ApiError::NotFound(not_found.into()))?;
    let expires = Utc::now().timestamp() + LINK_TTL_SECONDS;
    Ok(SignedUrl {
        url: format!("{origin}{}", signer.signed_path(key, expires)),
        content_type: info
            .content_type
            .unwrap_or_else(|| DEFAULT_CONTENT_TYPE.into()),
    })
}

/// A link to the receipt `receipts/<tenant_name>/<filename>`.
pub async fn receipt_link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    tenant_name: &str,
    filename: &str,
) -> Result<SignedUrl, ApiError> {
    let key = format!("receipts/{tenant_name}/{filename}");
    link(files, signer, origin, &key, "Receipt not found").await
}

/// A link to a bill's payment image `tenant-payments/<tenant_name>/<filename>`.
pub async fn tenant_payment_link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    tenant_name: &str,
    filename: &str,
) -> Result<SignedUrl, ApiError> {
    let key = format!("tenant-payments/{tenant_name}/{filename}");
    link(files, signer, origin, &key, "Payment image not found").await
}

/// A link to the payment image `payments/<name>.png`.
pub async fn payment_link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    name: &str,
) -> Result<SignedUrl, ApiError> {
    let key = format!("payments/{name}.png");
    link(files, signer, origin, &key, "Payment image not found").await
}
