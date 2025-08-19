use crate::entities::electricity_reading;
use crate::services::electricity_reading_service;
use axum::{Extension, Json, extract::Path, http::StatusCode};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ReadingInput {
    pub tenant_id: i32,
    pub room_id: i32,
    pub prev_reading: i32,
    pub curr_reading: i32,
}

/// GET /readings
pub async fn get_readings(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<electricity_reading::Model>>, StatusCode> {
    electricity_reading_service::get_all_readings(&db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// GET /readings/:id
pub async fn get_reading(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<(StatusCode, Json<electricity_reading::Model>), StatusCode> {
    match electricity_reading_service::get_reading_by_id(&db, id).await {
        Ok(Some(r)) => Ok((StatusCode::OK, Json(r))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /readings
pub async fn create_reading(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<ReadingInput>,
) -> Result<(StatusCode, Json<electricity_reading::Model>), StatusCode> {
    let active_model = electricity_reading::ActiveModel {
        tenant_id: Set(payload.tenant_id),
        room_id: Set(payload.room_id),
        prev_reading: Set(payload.prev_reading),
        curr_reading: Set(payload.curr_reading),
        ..Default::default()
    };

    electricity_reading_service::create_reading(&db, active_model)
        .await
        .map(|reading| (StatusCode::CREATED, Json(reading)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /readings/:id
pub async fn update_reading(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<ReadingInput>,
) -> Result<(StatusCode, Json<electricity_reading::Model>), StatusCode> {
    let active_model = electricity_reading::ActiveModel {
        id: Set(id),
        tenant_id: Set(payload.tenant_id),
        room_id: Set(payload.room_id),
        prev_reading: Set(payload.prev_reading),
        curr_reading: Set(payload.curr_reading),
        ..Default::default()
    };

    match electricity_reading_service::update_reading(&db, id, active_model).await {
        Ok(updated) => Ok((StatusCode::OK, Json(updated))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /readings/:id
pub async fn delete_reading(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match electricity_reading_service::delete_reading(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
