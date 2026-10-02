use axum::{Json, extract::State, http::StatusCode};
use m18_residences_db::entities::room;
use m18_residences_shared_rs::{auth::Admin, error::ApiError};
use sea_orm::ActiveValue::Set;
use serde::Deserialize;

use crate::app::AppState;
use crate::property::handlers::{IdParam, require_name};
use crate::property::services::room_service;
use m18_residences_shared_rs::extract::{ValidJson, ValidPath};

#[derive(Deserialize)]
pub struct RoomInput {
    pub name: String,
    pub rent: i32,
}

impl RoomInput {
    fn into_active_model(self) -> Result<room::ActiveModel, ApiError> {
        require_name(&self.name)?;
        Ok(room::ActiveModel {
            name: Set(self.name),
            rent: Set(self.rent),
            ..Default::default()
        })
    }
}

/// GET /rooms (admin)
pub async fn get_rooms(
    _admin: Admin,
    State(state): State<AppState>,
) -> Result<Json<Vec<room::Model>>, ApiError> {
    room_service::get_all_rooms(&state.db).await.map(Json)
}

/// GET /rooms/{id} (admin)
pub async fn get_room(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<Json<room::Model>, ApiError> {
    room_service::get_room_by_id(&state.db, id).await.map(Json)
}

/// POST /rooms (admin)
pub async fn create_room(
    _admin: Admin,
    State(state): State<AppState>,
    ValidJson(payload): ValidJson<RoomInput>,
) -> Result<(StatusCode, Json<room::Model>), ApiError> {
    let item = payload.into_active_model()?;
    let room = room_service::create_room(&state.db, item).await?;
    Ok((StatusCode::CREATED, Json(room)))
}

/// PUT /rooms/{id} (admin): replaces name and rent.
pub async fn update_room(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
    ValidJson(payload): ValidJson<RoomInput>,
) -> Result<Json<room::Model>, ApiError> {
    let item = payload.into_active_model()?;
    room_service::update_room(&state.db, id, item)
        .await
        .map(Json)
}

/// DELETE /rooms/{id} (admin)
pub async fn delete_room(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<StatusCode, ApiError> {
    room_service::delete_room(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
