//! Serving stored files behind signed links.
use chrono::Utc;
use m18_residences_shared_rs::{
    error::ApiError,
    files::{FileSigner, FileStore, StoredFile},
};

/// The file behind a signed link, if the link is valid and unexpired.
pub async fn get_signed_file(
    files: &dyn FileStore,
    signer: &FileSigner,
    key: &str,
    expires: Option<i64>,
    signature: Option<&str>,
) -> Result<StoredFile, ApiError> {
    let valid = match (expires, signature) {
        (Some(expires), Some(signature)) => {
            signer.verify(key, expires, signature, Utc::now().timestamp())
        }
        _ => false,
    };
    if !valid {
        return Err(ApiError::Forbidden("Invalid or expired link".into()));
    }
    files
        .get(key)
        .await?
        .ok_or_else(|| ApiError::NotFound("File not found".into()))
}
