use crate::entities::room;
use crate::services::room_service;
use axum::{Extension, Json, extract::Path, http::StatusCode};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RoomInput {
    pub name: String,
    pub rent: i32,
}

/// GET /rooms
pub async fn get_rooms(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<room::Model>>, StatusCode> {
    let rooms = room_service::get_all_rooms(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rooms))
}

/// GET /rooms/:id
pub async fn get_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<(StatusCode, Json<room::Model>), StatusCode> {
    match room_service::get_room_by_id(&db, id).await {
        Ok(Some(r)) => Ok((StatusCode::OK, Json(r))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /rooms
pub async fn create_room(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<RoomInput>,
) -> Result<(StatusCode, Json<room::Model>), StatusCode> {
    let active_model = room::ActiveModel {
        name: Set(payload.name),
        rent: Set(payload.rent),
        ..Default::default()
    };

    room_service::create_room(&db, active_model)
        .await
        .map(|room| (StatusCode::CREATED, Json(room)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /rooms/:id
pub async fn update_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<RoomInput>,
) -> Result<(StatusCode, Json<room::Model>), StatusCode> {
    let active_model = room::ActiveModel {
        id: Set(id),
        name: Set(payload.name),
        rent: Set(payload.rent),
        ..Default::default()
    };

    match room_service::update_room(&db, id, active_model).await {
        Ok(updated) => Ok((StatusCode::OK, Json(updated))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /rooms/:id
pub async fn delete_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match room_service::delete_room(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
