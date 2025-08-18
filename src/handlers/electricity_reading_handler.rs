use axum::{extract::Path, Extension, Json, http::StatusCode};
use sea_orm::DatabaseConnection;
use crate::entities::electricity_reading;
use crate::repository::electricity_reading_repo;
use crate::services::electricity_reading_service;
use serde::Deserialize;
use sea_orm::ActiveValue::Set;

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
    let readings = electricity_reading_repo::get_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(readings))
}

/// GET /readings/:id
pub async fn get_reading(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<electricity_reading::Model>, StatusCode> {
    match electricity_reading_repo::get_by_id(&db, id).await {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /readings
pub async fn create_reading(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<ReadingInput>,
) -> Result<Json<electricity_reading::Model>, StatusCode> {
    let active_model = electricity_reading::ActiveModel {
        tenant_id: Set(payload.tenant_id),
        room_id: Set(payload.room_id),
        prev_reading: Set(payload.prev_reading),
        curr_reading: Set(payload.curr_reading),
        ..Default::default()
    };

    electricity_reading_service::create_reading(&db, active_model)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /readings/:id
pub async fn update_reading(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<ReadingInput>,
) -> Result<Json<electricity_reading::Model>, StatusCode> {
    let active_model = electricity_reading::ActiveModel {
        id: Set(id),
        tenant_id: Set(payload.tenant_id),
        room_id: Set(payload.room_id),
        prev_reading: Set(payload.prev_reading),
        curr_reading: Set(payload.curr_reading),
        ..Default::default()
    };

    electricity_reading_service::update_reading(&db, id, active_model)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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