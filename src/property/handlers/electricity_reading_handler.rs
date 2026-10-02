use axum::{Json, extract::State, http::StatusCode};
use m18_residences_db::entities::electricity_reading;
use m18_residences_shared_rs::{auth::Admin, error::ApiError};
use sea_orm::ActiveValue::Set;
use serde::Deserialize;

use crate::app::AppState;
use crate::property::handlers::{IdParam, ValidJson, ValidPath};
use crate::property::services::electricity_reading_service;

/// `consumption` is computed by the server; one sent here is ignored.
#[derive(Deserialize)]
pub struct ReadingInput {
    pub tenant_id: i32,
    pub room_id: i32,
    pub prev_reading: i32,
    pub curr_reading: i32,
}

impl ReadingInput {
    fn into_active_model(self) -> electricity_reading::ActiveModel {
        electricity_reading::ActiveModel {
            tenant_id: Set(self.tenant_id),
            room_id: Set(self.room_id),
            prev_reading: Set(self.prev_reading),
            curr_reading: Set(self.curr_reading),
            ..Default::default()
        }
    }
}

/// GET /electricity-readings (admin)
pub async fn get_readings(
    _admin: Admin,
    State(state): State<AppState>,
) -> Result<Json<Vec<electricity_reading::Model>>, ApiError> {
    electricity_reading_service::get_all_readings(&state.db)
        .await
        .map(Json)
}

/// GET /electricity-readings/{id} (admin)
pub async fn get_reading(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<Json<electricity_reading::Model>, ApiError> {
    electricity_reading_service::get_reading_by_id(&state.db, id)
        .await
        .map(Json)
}

/// POST /electricity-readings (admin)
pub async fn create_reading(
    _admin: Admin,
    State(state): State<AppState>,
    ValidJson(payload): ValidJson<ReadingInput>,
) -> Result<(StatusCode, Json<electricity_reading::Model>), ApiError> {
    let reading =
        electricity_reading_service::create_reading(&state.db, payload.into_active_model()).await?;
    Ok((StatusCode::CREATED, Json(reading)))
}

/// PUT /electricity-readings/{id} (admin): replaces all four fields, recomputes consumption.
pub async fn update_reading(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
    ValidJson(payload): ValidJson<ReadingInput>,
) -> Result<Json<electricity_reading::Model>, ApiError> {
    electricity_reading_service::update_reading(&state.db, id, payload.into_active_model())
        .await
        .map(Json)
}

/// DELETE /electricity-readings/{id} (admin)
pub async fn delete_reading(
    _admin: Admin,
    State(state): State<AppState>,
    ValidPath(IdParam { id }): ValidPath<IdParam>,
) -> Result<StatusCode, ApiError> {
    electricity_reading_service::delete_reading(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
