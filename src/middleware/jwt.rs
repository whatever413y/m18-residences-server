use axum::{
    body::Body, http::{Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
}

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".into())
}

/// Middleware that ensures the request has a valid token
pub async fn require_auth(mut req: Request<Body>, next: Next) -> Response
{
    let claims = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .and_then(|h| {
            let token = h.trim_start_matches("Bearer ");
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(jwt_secret().as_bytes()),
                &Validation::new(Algorithm::HS256),
            )
            .ok()
            .map(|data| data.claims)
        });

    match claims {
        Some(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        None => {
            let body = Json(serde_json::json!({ "error": "Authentication required" }));
            (StatusCode::UNAUTHORIZED, body).into_response()
        }
    }
}

/// Middleware that ensures the request has admin claims
pub async fn require_admin(
    req: Request<Body>,
    next: Next,
) -> Response {
    // Extract Claims from request extensions
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned();

    match claims {
        Some(claims) if claims.role.as_deref() == Some("admin") => next.run(req).await,
        _ => {
            let body = Json(serde_json::json!({ "error": "Admin access required" }));
            (axum::http::StatusCode::FORBIDDEN, body).into_response()
        }
    }
}