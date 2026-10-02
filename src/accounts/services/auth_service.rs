//! Issues and validates JWTs. Admins log in with the configured credentials,
//! tenants with their exact name; tokens are stateless.
use chrono::Utc;
use m18_residences_db::entities::tenant::Model as Tenant;
use m18_residences_shared_rs::{
    auth::{ADMIN_ROLE, Claims, JwtKeys},
    error::ApiError,
    log_ok,
};

use crate::accounts::repository::tenant_read_repo;
use crate::app::AppState;

/// Admin tokens last 1 hour.
pub const ADMIN_TOKEN_TTL_SECONDS: i64 = 3600;
/// Tenant tokens last 20 minutes.
pub const TENANT_TOKEN_TTL_SECONDS: i64 = 1200;

fn expires_in(seconds: i64) -> usize {
    (Utc::now().timestamp() + seconds) as usize
}

/// Admin login: exact, case-sensitive match with the configured credentials.
/// Returns the token.
pub fn admin_login(state: &AppState, username: &str, password: &str) -> Result<String, ApiError> {
    let config = &state.config;
    if username != config.admin_username || password != config.admin_password {
        return Err(ApiError::Unauthorized("Invalid credentials".into()));
    }
    let claims = Claims {
        id: None,
        name: Some(username.to_string()),
        role: Some(ADMIN_ROLE.to_string()),
        exp: expires_in(ADMIN_TOKEN_TTL_SECONDS),
    };
    let token = state.jwt.issue(&claims)?;
    log_ok!("admin_login: admin logged in");
    Ok(token)
}

/// Tenant login by exact name (inactive tenants included, as before).
pub async fn tenant_login(state: &AppState, name: &str) -> Result<(String, Tenant), ApiError> {
    let tenant = tenant_read_repo::find_by_name(state.db.conn(), name)
        .await?
        .ok_or_else(|| ApiError::NotFound("Tenant not found".into()))?;
    let claims = Claims {
        id: Some(tenant.id),
        name: Some(tenant.name.clone()),
        role: None,
        exp: expires_in(TENANT_TOKEN_TTL_SECONDS),
    };
    let token = state.jwt.issue(&claims)?;
    log_ok!("tenant_login: tenant id={} logged in", tenant.id);
    Ok((token, tenant))
}

/// The claims of an `Authorization: Bearer <token>` header value.
pub fn validate_token(state: &AppState, header: Option<&str>) -> Result<Claims, ApiError> {
    let token =
        JwtKeys::bearer(header).ok_or_else(|| ApiError::Unauthorized("Missing token".into()))?;
    state
        .jwt
        .verify(token)
        .ok_or_else(|| ApiError::Unauthorized("Invalid token".into()))
}
