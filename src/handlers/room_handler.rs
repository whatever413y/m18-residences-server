use axum::{extract::Path, Extension, Json, http::StatusCode};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use crate::entities::room;
use crate::repository::room_repo;
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
    let rooms = room_repo::get_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rooms))
}

/// GET /rooms/:id
pub async fn get_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<room::Model>, StatusCode> {
    match room_repo::get_by_id(&db, id).await {
        Ok(Some(room)) => Ok(Json(room)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /rooms
pub async fn create_room(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<RoomInput>,
) -> Result<Json<room::Model>, StatusCode> {
    let active_model = room::ActiveModel {
        name: Set(payload.name),
        rent: Set(payload.rent),
        ..Default::default()
    };

    let room = room_repo::create(&db, active_model)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(room))
}

/// PUT /rooms/:id
pub async fn update_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<RoomInput>,
) -> Result<Json<room::Model>, StatusCode> {
    let active_model = room::ActiveModel {
        id: Set(id),
        name: Set(payload.name),
        rent: Set(payload.rent),
        ..Default::default()
    };

    let room = room_repo::update(&db, id, active_model)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(room))
}

/// DELETE /rooms/:id
pub async fn delete_room(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match room_repo::delete(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
