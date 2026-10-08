//! Files the billing records no longer point at (a replaced, cleared or deleted bill's receipt or payment image, a
//! payment method's old QR image) are kept under `archive/<key>`, forever, never reachable through signed links.
//! A failed move is recorded in `file_cleanup` and retried by the daily scheduled run.
use m18_residences_db::Db;
use m18_residences_shared_rs::{
    files::{FileError, FileStore},
    log_error, log_ok, log_warn,
};

use crate::billing::{
    repository::file_cleanup_repo, services::signed_url_service::DEFAULT_CONTENT_TYPE,
};

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

/// Archives `key` (what the file was: `what`, e.g. "receipt of bill 7"), after the database stopped pointing at it.
/// A failure is logged and recorded for the daily retry, never returned: the database write already happened.
pub async fn archive_or_record(db: &Db, files: &dyn FileStore, key: &str, what: &str) {
    let err = match archive_file(files, key).await {
        Ok(()) => return log_ok!("Archived {what}: {key}"),
        Err(err) => err,
    };
    log_error!("Archiving {what} ({key}) failed, retried daily: {}", err.0);
    if let Err(record) = file_cleanup_repo::record(db.conn(), key, what, &err.0).await {
        log_error!("{key} ({what}) is orphaned: recording it for the retry failed too ({record})");
    }
}
