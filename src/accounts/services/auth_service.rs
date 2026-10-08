//! Issues and validates JWTs. Admins log in with the configured credentials,
//! tenants with their name in any case; tokens are stateless. Both logins are
//! rate limited per client and checked with the captcha first.
use chrono::Utc;
use m18_residences_db::entities::tenant::Model as Tenant;
use m18_residences_shared_rs::{
    auth::{ADMIN_ROLE, Claims, JwtKeys},
    error::ApiError,
    log_error, log_ok, log_warn,
};

use crate::accounts::repository::tenant_read_repo;
use crate::app::AppState;

/// Admin tokens last 1 hour.
pub const ADMIN_TOKEN_TTL_SECONDS: i64 = 3600;
/// Tenant tokens last 20 minutes.
pub const TENANT_TOKEN_TTL_SECONDS: i64 = 1200;

/// Which login an attempt is for: each is limited on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Login {
    Admin,
    Tenant,
}

impl Login {
    fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin-login",
            Self::Tenant => "login",
        }
    }
}

/// Checks a login attempt before its credentials: at most the limiter's count
/// of attempts per client (`ip`; 429 over it), then the Turnstile `token`.
/// A failing limiter, or Turnstile being unreachable (an outage), lets the
/// attempt through, logged: an outage must not lock everyone out, and the
/// rate limit still applies. A token Cloudflare rejects is always refused.
/// Until both apps send tokens, a missing token is allowed.
pub async fn check_login_guards(
    state: &AppState,
    login: Login,
    ip: Option<&str>,
    token: Option<&str>,
) -> Result<(), ApiError> {
    let key = format!("{}:{}", login.as_str(), ip.unwrap_or("unknown"));
    match state.login_guards.limiter.allow(&key).await {
        Ok(true) => {}
        Ok(false) => {
            log_warn!("{}: too many attempts from one client", login.as_str());
            return Err(ApiError::TooManyRequests(
                "Too many attempts. Please wait a minute and try again.".into(),
            ));
        }
        Err(err) => log_error!(
            "{}: the rate limiter failed, letting the attempt through ({})",
            login.as_str(),
            err.0
        ),
    }
    let Some(token) = token else {
        return Ok(());
    };
    match state.login_guards.captcha.verify(token, ip).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(ApiError::BadRequest(
            "Verification failed. Please try again.".into(),
        )),
        Err(err) => {
            log_error!(
                "{}: Turnstile could not be reached, letting the attempt through ({})",
                login.as_str(),
                err.0
            );
            Ok(())
        }
    }
}

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

/// Tenant login by name, ignoring surrounding spaces and case (inactive tenants
/// included, as before). If two names differ only by case (data from before
/// migration 0006), only the exact one logs in.
pub async fn tenant_login(state: &AppState, name: &str) -> Result<(String, Tenant), ApiError> {
    let name = name.trim();
    let mut found = tenant_read_repo::find_by_name_ignoring_case(state.db.conn(), name).await?;
    let tenant = if found.len() == 1 {
        found.pop()
    } else {
        found.into_iter().find(|t| t.name == name)
    }
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
