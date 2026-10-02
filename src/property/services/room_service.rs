use m18_residences_db::{Db, entities::room};
use m18_residences_shared_rs::{error::ApiError, log_ok};
use sea_orm::DbErr;

use crate::property::repository::room_repo;
use crate::property::services::{Violation, violation};

fn not_found(id: i32) -> ApiError {
    ApiError::NotFound(format!("Room {id} not found"))
}

/// A failed insert/update: a taken name is a 409 that says so; the rest by the default mapping.
fn write_error(err: DbErr, item: &room::ActiveModel) -> ApiError {
    match (violation(&err), item.name.try_as_ref()) {
        (Some(Violation::Unique), Some(name)) => {
            ApiError::Conflict(format!("A room named \"{name}\" already exists"))
        }
        _ => err.into(),
    }
}

/// Get all rooms, by name.
pub async fn get_all_rooms(db: &Db) -> Result<Vec<room::Model>, ApiError> {
    let rooms = room_repo::get_all(db.conn()).await?;
    log_ok!("get_all_rooms: fetched {} rooms", rooms.len());
    Ok(rooms)
}

/// Get room by ID (404 if missing).
pub async fn get_room_by_id(db: &Db, id: i32) -> Result<room::Model, ApiError> {
    let room = room_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    log_ok!(
        "get_room_by_id: found room id={} name={}",
        room.id,
        room.name
    );
    Ok(room)
}

/// Create room (409 if the name is taken).
pub async fn create_room(db: &Db, item: room::ActiveModel) -> Result<room::Model, ApiError> {
    let room = room_repo::create(db.conn(), item.clone())
        .await
        .map_err(|err| write_error(err, &item))?;
    log_ok!(
        "create_room: created room id={} name={}",
        room.id,
        room.name
    );
    Ok(room)
}

/// Update room's name and rent (404 if missing, 409 if the name is taken).
/// `updated_at` is left as is.
pub async fn update_room(
    db: &Db,
    id: i32,
    item: room::ActiveModel,
) -> Result<room::Model, ApiError> {
    let room = room_repo::update(db.conn(), id, item.clone())
        .await
        .map_err(|err| match err {
            DbErr::RecordNotUpdated => not_found(id),
            err => write_error(err, &item),
        })?;
    log_ok!(
        "update_room: updated room id={} name={}",
        room.id,
        room.name
    );
    Ok(room)
}

/// Delete room (404 if missing, 409 while tenants or readings refer to it).
pub async fn delete_room(db: &Db, id: i32) -> Result<room::Model, ApiError> {
    let room = room_repo::delete(db.conn(), id)
        .await
        .map_err(|err| match violation(&err) {
            Some(Violation::ForeignKey) => {
                ApiError::Conflict(format!("Room {id} still has tenants or readings"))
            }
            _ => err.into(),
        })?
        .ok_or_else(|| not_found(id))?;
    log_ok!(
        "delete_room: deleted room id={} name={}",
        room.id,
        room.name
    );
    Ok(room)
}
