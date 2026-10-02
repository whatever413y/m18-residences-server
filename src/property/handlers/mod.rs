pub mod electricity_reading_handler;
pub mod room_handler;
pub mod tenant_handler;

use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Request, rejection::JsonRejection},
    http::{StatusCode, request::Parts},
};
use m18_residences_shared_rs::error::ApiError;
use serde::{Deserialize, de::DeserializeOwned};

/// `Path<T>` whose rejection is a JSON 400 naming the parameter
/// (e.g. "Invalid URL: Cannot parse `id` with value `abc` to a `i32`").
pub struct ValidPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ValidPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(value)| Self(value))
            .map_err(|rejection| ApiError::BadRequest(rejection.body_text()))
    }
}

/// `Json<T>` whose rejections are JSON errors: 413 for an oversized body, 400
/// otherwise (wrong content type, bad syntax, or a field of the wrong shape,
/// which serde names, e.g. "missing field `rent`").
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, ApiError> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                Err(ApiError::PayloadTooLarge(rejection.body_text()))
            }
            Err(rejection) => Err(ApiError::BadRequest(rejection.body_text())),
        }
    }
}

/// The `{id}` path parameter.
#[derive(Deserialize)]
pub struct IdParam {
    pub id: i32,
}

/// A name must have some non-whitespace text (it is stored as given).
pub fn require_name(name: &str) -> Result<(), ApiError> {
    if name.trim().is_empty() {
        Err(ApiError::BadRequest("name must not be empty".into()))
    } else {
        Ok(())
    }
}
