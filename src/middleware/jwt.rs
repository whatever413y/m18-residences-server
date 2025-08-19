use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
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

fn error_response(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(serde_json::json!({ "error": msg.into() }))).into_response()
}

pub async fn require_auth(mut req: Request<Body>, next: Next) -> Response {
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
            req.extensions_mut().insert(claims.clone());

            if req.uri().path().starts_with("/api/admin")
                && claims.role.as_deref() != Some("admin")
            {
                return error_response(StatusCode::FORBIDDEN, "Admin access required");
            }

            next.run(req).await
        }
        None => error_response(StatusCode::UNAUTHORIZED, "Authentication required"),
    }
}
