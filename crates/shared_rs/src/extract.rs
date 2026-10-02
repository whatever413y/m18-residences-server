//! Request extractors whose rejections are JSON [`ApiError`]s instead of
//! Axum's plain-text ones, so every error the API returns has the same shape.
use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Path, Request, rejection::JsonRejection},
    http::{StatusCode, request::Parts},
};
use serde::de::DeserializeOwned;

use crate::error::ApiError;

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
