use axum::{
    extract::{Extension, Json},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use crate::services::auth_service::{admin_login, tenant_login, validate_token};

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

pub async fn admin_login_handler(Json(input): Json<AdminLoginInput>) -> impl IntoResponse {
    match admin_login(&input.username, &input.password).await {
        Ok(token) => (
            StatusCode::OK,
            Json(TokenResponse {
                token,
                role: Some("admin".to_string()),
                username: Some(input.username),
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
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
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

pub async fn validate_token_handler(headers: HeaderMap) -> impl IntoResponse {
    match validate_token(&headers) {
        Ok(claims) => {
            (StatusCode::OK, Json(serde_json::json!({ "user": claims })))
        }
        Err((status, msg)) => (status, Json(serde_json::json!({ "error": msg }))),
    }
}