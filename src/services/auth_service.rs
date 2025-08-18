use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::DatabaseConnection;
use serde::{Serialize, Deserialize};
use chrono::Utc;

use crate::services::tenant_service;
use crate::entities::tenant::Model as Tenant;

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".into())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
}

/// Admin login
pub async fn admin_login(username: &str, password: &str) -> Result<String, String> {
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_default();
    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap_or_default();

    if username != admin_username || password != admin_password {
        return Err("Invalid admin credentials".into());
    }

    let claims = Claims {
        id: None,
        name: Some(username.to_string()),
        role: Some("admin".to_string()),
        exp: (Utc::now().timestamp() + 3600) as usize, // 1 hour
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(|e| e.to_string())
}

/// Tenant login
pub async fn tenant_login(
    db: &DatabaseConnection,
    name: &str,
) -> Result<(String, Tenant), String> {
    let tenant_opt = tenant_service::get_tenant_by_name(db, name)
        .await
        .map_err(|e| e.to_string())?;

    let tenant = tenant_opt.ok_or_else(|| "Tenant not found".to_string())?;

    let claims = Claims {
        id: Some(tenant.id),
        name: Some(tenant.name.clone()),
        role: None,
        exp: (Utc::now().timestamp() + 1200) as usize, // 20 minutes
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(|e| e.to_string())?;

    Ok((token, tenant))
}
