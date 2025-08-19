use axum::{
    extract::{Extension, Json},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse},
};
use serde::{Deserialize, Serialize};
use crate::services::auth_service::{admin_login, tenant_login, validate_token, AuthError};

#[derive(Deserialize)]
pub struct AdminLoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct TenantLoginInput {
    pub name: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub role: Option<String>,
    pub username: Option<String>,
}

fn map_auth_error(err: AuthError) -> (StatusCode, String) {
    match err {
        AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials".into()),
        AuthError::TenantNotFound => (StatusCode::NOT_FOUND, "Tenant not found".into()),
        AuthError::TokenMissing => (StatusCode::UNAUTHORIZED, "Missing token".into()),
        AuthError::TokenInvalid => (StatusCode::UNAUTHORIZED, "Invalid token".into()),
        AuthError::Other(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
    }
}

pub async fn admin_login_handler(Json(input): Json<AdminLoginInput>) -> impl IntoResponse {
    match admin_login(&input.username, &input.password).await {
        Ok(token) => (
            StatusCode::OK,
            Json(TokenResponse {
                token,
                role: Some("admin".into()),
                username: Some(input.username),
            }),
        )
            .into_response(),
        Err(err) => {
            let (status, msg) = map_auth_error(err);
            (status, Json(serde_json::json!({ "error": msg }))).into_response()
        }
    }
}

pub async fn tenant_login_handler(
    Extension(db): Extension<sea_orm::DatabaseConnection>,
    Json(input): Json<TenantLoginInput>,
) -> impl IntoResponse {
    match tenant_login(&db, &input.name).await {
        Ok((token, tenant)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "token": token, "tenant": tenant })),
        )
            .into_response(),
        Err(err) => {
            let (status, msg) = map_auth_error(err);
            (status, Json(serde_json::json!({ "error": msg }))).into_response()
        }
    }
}

pub async fn validate_token_handler(headers: HeaderMap) -> impl IntoResponse {
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

    match validate_token(auth_header) {
        Ok(claims) => (StatusCode::OK, Json(serde_json::json!({ "user": claims }))),
        Err(err) => {
            let (status, msg) = map_auth_error(err);
            (status, Json(serde_json::json!({ "error": msg })))
        }
    }
}
