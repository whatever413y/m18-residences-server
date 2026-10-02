use axum::{
    extract::{Query, State, rejection::QueryRejection},
    http::{
        HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    response::{IntoResponse, Response},
};
use m18_residences_shared_rs::{error::ApiError, extract::ValidPath, files::LINK_TTL_SECONDS};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::services::{file_service, signed_url_service::DEFAULT_CONTENT_TYPE},
};

#[derive(Deserialize)]
pub struct FileKey {
    pub key: String,
}

/// The signature query. Kept as text: anything malformed is an invalid link (403).
#[derive(Deserialize)]
pub struct LinkQuery {
    pub expires: Option<String>,
    pub signature: Option<String>,
}

/// GET /api/files/{*key}?expires=..&signature=.. (no token: the signed link is the permission)
pub async fn get_file(
    State(state): State<AppState>,
    path: ValidPath<FileKey>,
    query: Result<Query<LinkQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let ValidPath(FileKey { key }) = path;
    let (expires, signature) = match &query {
        Ok(Query(q)) => (
            q.expires.as_deref().and_then(|e| e.parse::<i64>().ok()),
            q.signature.as_deref(),
        ),
        Err(_) => (None, None),
    };
    let file = file_service::get_signed_file(
        state.files.as_ref(),
        &state.signer,
        &key,
        expires,
        signature,
    )
    .await?;

    let content_type = file
        .content_type
        .as_deref()
        .and_then(|t| HeaderValue::from_str(t).ok())
        .unwrap_or_else(|| HeaderValue::from_static(DEFAULT_CONTENT_TYPE));
    let cache_control = HeaderValue::from_str(&format!("private, max-age={LINK_TTL_SECONDS}"))
        .map_err(|e| ApiError::Internal(format!("Cache-Control header: {e}")))?;
    Ok((
        [
            (CONTENT_TYPE, content_type),
            (CACHE_CONTROL, cache_control),
            (X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
        ],
        file.bytes,
    )
        .into_response())
}
