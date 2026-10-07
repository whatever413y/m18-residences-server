//! Files the billing records no longer point at (a replaced, cleared or deleted bill's receipt or payment image, a
//! payment method's old QR image) are kept under `archive/<key>`, forever, never reachable through signed links.
use m18_residences_shared_rs::{
    files::{FileError, FileStore},
    log_warn,
};

use crate::billing::services::signed_url_service::DEFAULT_CONTENT_TYPE;

/// Where a file no longer in use is kept: `archive/<key>`.
pub fn archive_key(key: &str) -> String {
    format!("archive/{key}")
}

/// Moves `key` to [`archive_key`] (kept forever): a copy, then the original is
/// deleted, so a failure never loses the file. A file already gone is logged
/// and counts as done.
pub async fn archive_file(files: &dyn FileStore, key: &str) -> Result<(), FileError> {
    let Some(file) = files.get(key).await? else {
        log_warn!("Nothing to archive at {key}: the file is already gone");
        return Ok(());
    };
    let content_type = file.content_type.as_deref().unwrap_or(DEFAULT_CONTENT_TYPE);
    files
        .put(&archive_key(key), file.bytes, content_type)
        .await?;
    files.delete(key).await
}
