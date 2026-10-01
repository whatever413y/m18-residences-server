//! JWT verification and permissions (the diagram's BearerAuth + permissions).
//!
//! Tokens are HS256 with the claims the apps already hold, so existing tokens
//! stay valid: admins have `role: "admin"`; tenants have `role: null`, their
//! tenant `id` and `name`.
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;

pub const ADMIN_ROLE: &str = "admin";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
}

impl Claims {
    pub fn is_admin(&self) -> bool {
        self.role.as_deref() == Some(ADMIN_ROLE)
    }

    fn is_tenant(&self) -> bool {
        self.role.is_none()
    }

    /// Admins may access every tenant; a tenant only itself.
    pub fn ensure_admin_or_tenant(&self, tenant_id: i32) -> Result<(), ApiError> {
        if self.is_admin() || (self.is_tenant() && self.id == Some(tenant_id)) {
            Ok(())
        } else {
            Err(forbidden_other_tenant())
        }
    }

    /// Like [`Claims::ensure_admin_or_tenant`], for resources keyed by tenant name.
    pub fn ensure_admin_or_name(&self, tenant_name: &str) -> Result<(), ApiError> {
        if self.is_admin() || (self.is_tenant() && self.name.as_deref() == Some(tenant_name)) {
            Ok(())
        } else {
            Err(forbidden_other_tenant())
        }
    }
}

fn forbidden_other_tenant() -> ApiError {
    ApiError::Forbidden("You can only access your own records".into())
}

/// Signs and verifies tokens with the `JWT_SECRET`.
#[derive(Clone)]
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn issue(&self, claims: &Claims) -> Result<String, ApiError> {
        encode(&Header::new(Algorithm::HS256), claims, &self.encoding)
            .map_err(|e| ApiError::Internal(format!("signing a token: {e}")))
    }

    /// The claims of a valid, unexpired token.
    pub fn verify(&self, token: &str) -> Option<Claims> {
        decode::<Claims>(token, &self.decoding, &Validation::new(Algorithm::HS256))
            .ok()
            .map(|data| data.claims)
    }

    /// The token of an `Authorization: Bearer <token>` header value.
    pub fn bearer(header: Option<&str>) -> Option<&str> {
        header.and_then(|h| h.strip_prefix("Bearer "))
    }
}

/// Any logged-in user (401 otherwise).
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    JwtKeys: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        if let Some(claims) = parts.extensions.get::<Claims>() {
            return Ok(Self(claims.clone()));
        }
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());
        let claims = JwtKeys::bearer(header)
            .and_then(|token| JwtKeys::from_ref(state).verify(token))
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".into()))?;
        parts.extensions.insert(claims.clone());
        Ok(Self(claims))
    }
}

/// An admin (401 without a valid token, 403 for tenants).
pub struct Admin(pub Claims);

impl<S> FromRequestParts<S> for Admin
where
    JwtKeys: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        if claims.is_admin() {
            Ok(Self(claims))
        } else {
            Err(ApiError::Forbidden("Admin access required".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant(id: i32, name: &str) -> Claims {
        Claims {
            id: Some(id),
            name: Some(name.into()),
            role: None,
            exp: usize::MAX / 2,
        }
    }

    fn admin() -> Claims {
        Claims {
            id: None,
            name: Some("boss".into()),
            role: Some(ADMIN_ROLE.into()),
            exp: usize::MAX / 2,
        }
    }

    #[test]
    fn round_trips_and_rejects_bad_tokens() {
        let keys = JwtKeys::new("secret-a");
        let token = keys.issue(&tenant(7, "ANA")).unwrap();
        assert_eq!(keys.verify(&token), Some(tenant(7, "ANA")));
        assert_eq!(
            JwtKeys::new("secret-b").verify(&token),
            None,
            "another secret"
        );
        assert_eq!(keys.verify("not.a.token"), None);

        let expired = Claims {
            exp: 1_000,
            ..tenant(7, "ANA")
        };
        assert_eq!(keys.verify(&keys.issue(&expired).unwrap()), None, "expired");
    }

    #[test]
    fn parses_bearer_headers() {
        assert_eq!(JwtKeys::bearer(Some("Bearer abc")), Some("abc"));
        assert_eq!(JwtKeys::bearer(Some("bearer abc")), None);
        assert_eq!(JwtKeys::bearer(None), None);
    }

    #[test]
    fn tenants_reach_only_their_own_records() {
        let ana = tenant(7, "ANA");
        assert!(ana.ensure_admin_or_tenant(7).is_ok());
        assert!(ana.ensure_admin_or_tenant(8).is_err());
        assert!(ana.ensure_admin_or_name("ANA").is_ok());
        assert!(ana.ensure_admin_or_name("BEN").is_err());
        assert!(admin().ensure_admin_or_tenant(8).is_ok());
        assert!(admin().ensure_admin_or_name("BEN").is_ok());

        // A token with some other role is neither admin nor tenant.
        let odd = Claims {
            role: Some("viewer".into()),
            ..tenant(7, "ANA")
        };
        assert!(odd.ensure_admin_or_tenant(7).is_err());
    }
}
