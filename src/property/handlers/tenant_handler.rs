use axum::{Json, extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use m18_residences_db::entities::tenant;
use m18_residences_shared_rs::{
    auth::{Admin, AuthUser},
    error::ApiError,
};
use sea_orm::ActiveValue::Set;
use serde::Deserialize;

use crate::app::AppState;
use crate::property::handlers::{IdParam, tenant_name};
use crate::property::services::tenant_service;
use m18_residences_shared_rs::extract::{ValidJson, ValidPath};

#[derive(Deserialize)]
pub struct TenantInput {
    pub name: String,
    pub room_id: i32,
    /// `YYYY-MM-DDTHH:MM:SS[.f]`, no zone.
    pub join_date: NaiveDateTime,
    /// Missing or null means active (on update too).
    pub is_active: Option<bool>,
}

impl TenantInput {
    fn into_active_model(self) -> Result<tenant::ActiveModel, ApiError> {
        Ok(tenant::ActiveModel {
            name: Set(tenant_name(&self.name)?),
            room_id: Set(self.room_id),
            join_date: Set(self.join_date),
            is_active: Set(self.is_active.unwrap_or(true)),
            ..Default::default()
        })
    }
}

#[derive(Deserialize)]
pub struct NameParam {
    pub name: String,
}

/// GET /tenants (admin)
pub async fn get_tenants(
    _admin: Admin,
    State(state): State<AppState>,
) -> Result<Json<Vec<tenant::Model>>, ApiError> {
    tenant_service::get_all_tenants(&state.db).await.map(Json)
}

/// GET /tenants/{id} (admin, or that tenant)
pub async fn get_tenant(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<Json<tenant::Model>, ApiError> {
    claims.ensure_admin_or_tenant(id)?;
    tenant_service::get_tenant_by_id(&state.db, id)
        .await
        .map(Json)
}

/// GET /tenants/tenant/{name} (admin): exact name match.
pub async fn get_tenant_by_name(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(NameParam { name }): ValidPath<NameParam>,
) -> Result<Json<tenant::Model>, ApiError> {
    tenant_service::get_tenant_by_name(&state.db, &name)
        .await
        .map(Json)
}

/// POST /tenants (admin)
pub async fn create_tenant(
    _admin: Admin,
    State(state): State<AppState>,
    ValidJson(payload): ValidJson<TenantInput>,
) -> Result<(StatusCode, Json<tenant::Model>), ApiError> {
    let item = payload.into_active_model()?;
    let tenant = tenant_service::create_tenant(&state.db, item).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}

/// PUT /tenants/{id} (admin): replaces name, room, join date and active flag.
pub async fn update_tenant(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
    ValidJson(payload): ValidJson<TenantInput>,
) -> Result<Json<tenant::Model>, ApiError> {
    let item = payload.into_active_model()?;
    tenant_service::update_tenant(&state.db, id, item)
        .await
        .map(Json)
}

/// DELETE /tenants/{id} (admin)
pub async fn delete_tenant(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<StatusCode, ApiError> {
    tenant_service::delete_tenant(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
