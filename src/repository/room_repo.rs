use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, DbErr, QueryOrder};
use chrono::Utc;
use crate::entities::room;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<room::Model>, DbErr> {
    let rooms = room::Entity::find()
        .order_by_asc(room::Column::Name)
        .all(db)
        .await;

    match &rooms {
        Ok(list) => println!("✅ get_all: fetched {} rooms", list.len()),
        Err(err) => eprintln!("❌ get_all: error fetching rooms: {:?}", err),
    }

    rooms
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    let result = room::Entity::find_by_id(id).one(db).await;

    match &result {
        Ok(Some(room)) => println!("✅ get_by_id: found room id={} name={}", room.id, room.name),
        Ok(None) => println!("⚠️ get_by_id: room id={} not found", id),
        Err(err) => eprintln!("❌ get_by_id: error fetching room id={}: {:?}", id, err),
    }

    result
}

pub async fn create(db: &DatabaseConnection, mut item: room::ActiveModel) -> Result<room::Model, DbErr> {
    let now = Utc::now();
    item.created_at = Set(now.naive_utc());
    item.updated_at = Set(now.naive_utc());

    let result = item.insert(db).await;

    match &result {
        Ok(room) => println!("✅ create: created room id={} name={}", room.id, room.name),
        Err(err) => eprintln!("❌ create: error creating room: {:?}", err),
    }

    result
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: room::ActiveModel) -> Result<room::Model, DbErr> {
    item.id = Set(id);
    item.updated_at = Set(Utc::now().naive_utc());

    let result = item.update(db).await;

    match &result {
        Ok(room) => println!("✅ update: updated room id={} name={}", room.id, room.name),
        Err(err) => eprintln!("❌ update: error updating room id={}: {:?}", id, err),
    }

    result
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    if let Some(model) = room::Entity::find_by_id(id).one(db).await? {
        let am: room::ActiveModel = model.clone().into();
        let res = am.delete(db).await;

        match &res {
            Ok(_) => println!("✅ delete: deleted room id={} name={}", model.id, model.name),
            Err(err) => eprintln!("❌ delete: error deleting room id={}: {:?}", id, err),
        }

        res.map(|_| Some(model))
    } else {
        println!("⚠️ delete: room id={} not found", id);
        Ok(None)
    }
}
