//! The one error type handlers return. Every error response is JSON
//! `{"error": "<message>"}`, which the Flutter apps' `ApiException.message` shows.
use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::RETRY_AFTER},
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use serde_json::json;

use crate::{log_error, log_warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// 400: the request itself is wrong; the message says how.
    BadRequest(String),
    /// 401: no valid token.
    Unauthorized(String),
    /// 403: valid token, not allowed.
    Forbidden(String),
    /// 404
    NotFound(String),
    /// 409: conflicts with existing data (unique names, records still in use, missing references).
    Conflict(String),
    /// 413
    PayloadTooLarge(String),
    /// 429 with `Retry-After`: too many attempts; the message says so.
    TooManyRequests(String),
    /// 502: file storage failed. The detail is logged, not sent.
    Upstream(String),
    /// 500. The detail is logged, not sent.
    Internal(String),
    /// 503: the captcha check (Cloudflare Turnstile) could not be reached. The detail is logged, not sent.
    VerificationUnavailable(String),
}

/// Seconds a client is told to wait after a 429 (the login limiter's window).
pub const RETRY_AFTER_SECONDS: u32 = 60;

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::VerificationUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// What the client sees. Server-side failures get a generic message; their detail goes to the log.
    pub fn public_message(&self) -> &str {
        match self {
            Self::BadRequest(m)
            | Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::Conflict(m)
            | Self::PayloadTooLarge(m)
            | Self::TooManyRequests(m) => m,
            Self::Upstream(_) => "File storage is unavailable, try again",
            Self::Internal(_) => "Internal server error",
            Self::VerificationUnavailable(_) => "Verification is unavailable, try again",
        }
    }

    fn detail(&self) -> &str {
        match self {
            Self::BadRequest(m)
            | Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::NotFound(m)
            | Self::Conflict(m)
            | Self::PayloadTooLarge(m)
            | Self::TooManyRequests(m)
            | Self::Upstream(m)
            | Self::Internal(m)
            | Self::VerificationUnavailable(m) => m,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status(), self.detail())
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            log_error!("{status}: {}", self.detail());
        } else {
            log_warn!("{status}: {}", self.detail());
        }
        let mut response =
            (status, Json(json!({ "error": self.public_message() }))).into_response();
        if status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(RETRY_AFTER, HeaderValue::from(RETRY_AFTER_SECONDS));
        }
        response
    }
}

/// Which constraint a failed write broke. SQLite (D1 and the native tests)
/// reports constraint failures only in the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    Unique,
    ForeignKey,
}

pub fn violation(err: &DbErr) -> Option<Violation> {
    let text = err.to_string();
    if text.contains("UNIQUE constraint failed") {
        Some(Violation::Unique)
    } else if text.contains("FOREIGN KEY constraint failed") {
        Some(Violation::ForeignKey)
    } else {
        None
    }
}

/// Database errors by what they mean for the client. SQLite (D1 and the native
/// tests) reports constraint failures only in the message text.
impl From<DbErr> for ApiError {
    fn from(err: DbErr) -> Self {
        let text = err.to_string();
        match err {
            DbErr::RecordNotFound(_) | DbErr::RecordNotUpdated => Self::NotFound("Not found".into()),
            _ if text.contains("FOREIGN KEY constraint failed") => {
                Self::Conflict("This change conflicts with related records (still in use, or refers to one that doesn't exist)".into())
            }
            _ if text.contains("UNIQUE constraint failed") => Self::Conflict("A record with the same unique value already exists".into()),
            _ => Self::Internal(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    #[test]
    fn maps_database_errors() {
        assert_eq!(
            ApiError::from(DbErr::RecordNotUpdated).status(),
            StatusCode::NOT_FOUND
        );
        let fk = DbErr::Custom(
            "D1: Error: D1_ERROR: FOREIGN KEY constraint failed: SQLITE_CONSTRAINT".into(),
        );
        assert_eq!(ApiError::from(fk).status(), StatusCode::CONFLICT);
        let unique = DbErr::Custom("UNIQUE constraint failed: room.name".into());
        assert_eq!(ApiError::from(unique).status(), StatusCode::CONFLICT);
        assert_eq!(
            ApiError::from(DbErr::Custom("disk on fire".into())).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn a_429_says_when_to_retry() {
        let response = ApiError::TooManyRequests("Too many attempts".into()).into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[RETRY_AFTER], "60");
    }

    #[tokio::test]
    async fn responds_with_a_json_error_and_hides_server_details() {
        let response = ApiError::Internal("secret detail".into()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            json!({ "error": "Internal server error" })
        );

        let response = ApiError::Conflict("Room still has tenants".into()).into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            json!({ "error": "Room still has tenants" })
        );
    }
}
