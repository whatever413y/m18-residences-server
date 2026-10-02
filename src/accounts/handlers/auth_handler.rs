use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header::AUTHORIZATION},
};
use m18_residences_shared_rs::{auth::ADMIN_ROLE, error::ApiError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::accounts::services::auth_service::{admin_login, tenant_login, validate_token};
use crate::app::AppState;

#[derive(Deserialize)]
pub struct AdminLoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct TenantLoginInput {
    pub name: String,
}

/// The admin login response; the field order is the wire order.
#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub role: String,
    pub username: String,
}

/// `POST /api/auth/admin-login`: 200 `{token, role, username}`, 401 "Invalid credentials".
pub async fn admin_login_handler(
    State(state): State<AppState>,
    Json(input): Json<AdminLoginInput>,
) -> Result<Json<TokenResponse>, ApiError> {
    let token = admin_login(&state, &input.username, &input.password)?;
    Ok(Json(TokenResponse {
        token,
        role: ADMIN_ROLE.to_string(),
        username: input.username,
    }))
}

/// `POST /api/auth/login`: 200 `{tenant, token}`, 404 "Tenant not found".
pub async fn tenant_login_handler(
    State(state): State<AppState>,
    Json(input): Json<TenantLoginInput>,
) -> Result<Json<Value>, ApiError> {
    let (token, tenant) = tenant_login(&state, &input.name).await?;
    Ok(Json(json!({ "token": token, "tenant": tenant })))
}

/// `POST /api/auth/validate-token`: 200 `{user: claims}`, 401 "Missing token" / "Invalid token".
/// A header that isn't visible ASCII counts as missing.
pub async fn validate_token_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let header = headers.get(AUTHORIZATION).and_then(|h| h.to_str().ok());
    let claims = validate_token(&state, header)?;
    Ok(Json(json!({ "user": claims })))
}
