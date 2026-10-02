pub mod bill_handler;
pub mod file_handler;
pub mod signed_url_handler;

use axum::{
    extract::rejection::{JsonRejection, PathRejection},
    http::StatusCode,
};
use m18_residences_shared_rs::error::ApiError;

/// A rejected JSON body as a JSON error (axum's message names the field).
pub fn json_error(rejection: JsonRejection) -> ApiError {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge(rejection.body_text())
    } else {
        ApiError::BadRequest(rejection.body_text())
    }
}

/// A rejected path parameter as a JSON 400 (axum's message names the parameter).
pub fn path_error(rejection: PathRejection) -> ApiError {
    ApiError::BadRequest(rejection.body_text())
}
