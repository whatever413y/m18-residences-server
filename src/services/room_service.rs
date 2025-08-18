use crate::entities::room;
use crate::repository::room_repo;
use sea_orm::{DatabaseConnection, DbErr};

/// Get all rooms
pub async fn get_all_rooms(db: &DatabaseConnection) -> Result<Vec<room::Model>, DbErr> {
    let result = room_repo::get_all(db).await;
    match &result {
        Ok(list) => println!("✅ get_all_rooms: fetched {} rooms", list.len()),
        Err(err) => eprintln!("❌ get_all_rooms: error fetching rooms: {:?}", err),
    }
    result
}

/// Get room by ID 
pub async fn get_room_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    let result = room_repo::get_by_id(db, id).await;
    match &result {
        Ok(Some(r)) => println!("✅ get_room_by_id: found room id={} name={}", r.id, r.name),
        Ok(None) => println!("⚠️ get_room_by_id: room id={} not found", id),
        Err(err) => eprintln!("❌ get_room_by_id: error fetching room id={}: {:?}", id, err),
    }
    result
}

/// Create room 
pub async fn create_room(db: &DatabaseConnection, item: room::ActiveModel) -> Result<room::Model, DbErr> {
    let result = room_repo::create(db, item).await;
    match &result {
        Ok(r) => println!("✅ create_room: created room id={} name={}", r.id, r.name),
        Err(err) => eprintln!("❌ create_room: error creating room: {:?}", err),
    }
    result
}

/// Update room 
pub async fn update_room(db: &DatabaseConnection, id: i32, item: room::ActiveModel) -> Result<room::Model, DbErr> {
    let result = room_repo::update(db, id, item).await;
    match &result {
        Ok(r) => println!("✅ update_room: updated room id={} name={}", r.id, r.name),
        Err(err) => eprintln!("❌ update_room: error updating room id={}: {:?}", id, err),
    }
    result
}

/// Delete room 
pub async fn delete_room(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    let result = room_repo::delete(db, id).await;
    match &result {
        Ok(Some(r)) => println!("✅ delete_room: deleted room id={} name={}", r.id, r.name),
        Ok(None) => println!("⚠️ delete_room: room id={} not found", id),
        Err(err) => eprintln!("❌ delete_room: error deleting room id={}: {:?}", id, err),
    }
    result
}
