use crate::entities::tenant;
use crate::services::tenant_service;
use axum::{Extension, Json, extract::Path, http::StatusCode};
use chrono::NaiveDateTime;
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TenantInput {
    pub name: String,
    pub room_id: i32,
    pub join_date: NaiveDateTime,
    pub is_active: Option<bool>,
}

/// GET /tenants
pub async fn get_tenants(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<tenant::Model>>, StatusCode> {
    let tenants = tenant_service::get_all_tenants(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(tenants))
}

/// GET /tenants/:id
pub async fn get_tenant(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<(StatusCode, Json<tenant::Model>), StatusCode> {
    match tenant_service::get_tenant_by_id(&db, id).await {
        Ok(Some(t)) => Ok((StatusCode::OK, Json(t))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tenants/by-name/:name
pub async fn get_tenant_by_name(
    Path(name): Path<String>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<(StatusCode, Json<tenant::Model>), StatusCode> {
    match tenant_service::get_tenant_by_name(&db, &name).await {
        Ok(Some(t)) => Ok((StatusCode::OK, Json(t))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /tenants
pub async fn create_tenant(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<TenantInput>,
) -> Result<(StatusCode, Json<tenant::Model>), StatusCode> {
    let active_model = tenant::ActiveModel {
        name: Set(payload.name),
        room_id: Set(payload.room_id),
        join_date: Set(payload.join_date),
        is_active: Set(payload.is_active.unwrap_or(true)),
        ..Default::default()
    };

    tenant_service::create_tenant(&db, active_model)
        .await
        .map(|tenant| (StatusCode::CREATED, Json(tenant)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /tenants/:id
pub async fn update_tenant(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<TenantInput>,
) -> Result<(StatusCode, Json<tenant::Model>), StatusCode> {
    let active_model = tenant::ActiveModel {
        id: Set(id),
        name: Set(payload.name),
        room_id: Set(payload.room_id),
        join_date: Set(payload.join_date),
        is_active: Set(payload.is_active.unwrap_or(true)),
        ..Default::default()
    };

    match tenant_service::update_tenant(&db, id, active_model).await {
        Ok(updated) => Ok((StatusCode::OK, Json(updated))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /tenants/:id
pub async fn delete_tenant(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match tenant_service::delete_tenant(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
