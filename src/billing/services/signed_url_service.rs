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

/// A link to a bill's receipt or payment image stored at `key`.
pub async fn bill_file_link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    key: &str,
    not_found: &str,
) -> Result<SignedUrl, ApiError> {
    link(files, signer, origin, key, not_found).await
}

/// A link to a payment method's QR image stored at `key`.
pub async fn payment_method_link(
    files: &dyn FileStore,
    signer: &FileSigner,
    origin: &str,
    key: &str,
) -> Result<SignedUrl, ApiError> {
    link(files, signer, origin, key, "Payment image not found").await
}
