//! The daily retry of failed archives (the Worker's scheduled run): each file recorded in `file_cleanup` is moved
//! to `archive/<key>` again; archived (or already gone) files are forgotten, the others counted and kept.
use m18_residences_db::Db;
use m18_residences_shared_rs::{error::ApiError, files::FileStore, log_error, log_ok, log_warn};

use crate::billing::{repository::file_cleanup_repo, services::archive::archive_file};

/// At most this many files per run, so one run stays well within the Worker's limits.
pub const RETRIES_PER_RUN: u64 = 50;

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RetryReport {
    pub archived: usize,
    pub failed: usize,
}

/// Retries up to [`RETRIES_PER_RUN`] recorded files, oldest first.
pub async fn retry_archives(db: &Db, files: &dyn FileStore) -> Result<RetryReport, ApiError> {
    let mut report = RetryReport::default();
    for row in file_cleanup_repo::oldest(db.conn(), RETRIES_PER_RUN).await? {
        match archive_file(files, &row.key).await {
            Ok(()) => {
                file_cleanup_repo::remove(db.conn(), &row.key).await?;
                log_ok!(
                    "Archived {} ({}) on retry {}",
                    row.key,
                    row.reason,
                    row.attempts + 1
                );
                report.archived += 1;
            }
            Err(err) => {
                file_cleanup_repo::retry_failed(db.conn(), &row.key, &err.0).await?;
                log_error!(
                    "{} ({}) is still orphaned after retry {}: {}",
                    row.key,
                    row.reason,
                    row.attempts + 1,
                    err.0
                );
                report.failed += 1;
            }
        }
    }
    if report != RetryReport::default() {
        log_warn!(
            "File cleanup: {} archived, {} still failing",
            report.archived,
            report.failed
        );
    }
    Ok(report)
}
