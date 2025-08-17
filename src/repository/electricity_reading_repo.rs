use crate::entities::electricity_reading;
use crate::services::electricity_reading_service;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder, Set};

/// GET all readings
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<electricity_reading::Model>, DbErr> {
    let result = electricity_reading::Entity::find()
        .order_by_desc(electricity_reading::Column::CreatedAt)
        .all(db)
        .await;

    match &result {
        Ok(list) => println!("✅ get_all: fetched {} readings", list.len()),
        Err(err) => eprintln!("❌ get_all: error fetching readings: {:?}", err),
    }

    result
}

/// GET reading by ID
pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    let result = electricity_reading::Entity::find_by_id(id).one(db).await;

    match &result {
        Ok(Some(r)) => println!("✅ get_by_id: found reading id={}", r.id),
        Ok(None) => println!("⚠️ get_by_id: reading id={} not found", id),
        Err(err) => eprintln!("❌ get_by_id: error fetching reading id={}: {:?}", id, err),
    }

    result
}

/// CREATE a new reading
pub async fn create(
    db: &DatabaseConnection,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    // Calculate consumption using service
    item.consumption = Set(electricity_reading_service::calculate_consumption(
        item.prev_reading.clone(),
        item.curr_reading.clone(),
    ));

    let now = Utc::now().naive_utc();
    item.created_at = Set(now);
    item.updated_at = Set(now);

    let result = item.insert(db).await;

    match &result {
        Ok(r) => println!("✅ create: created reading id={}", r.id),
        Err(err) => eprintln!("❌ create: error creating reading: {:?}", err),
    }

    result
}

/// UPDATE a reading
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.id = Set(id);

    // Recalculate consumption
    item.consumption = Set(electricity_reading_service::calculate_consumption(
        item.prev_reading.clone(),
        item.curr_reading.clone(),
    ));

    item.updated_at = Set(Utc::now().naive_utc());

    let result = item.update(db).await;

    match &result {
        Ok(r) => println!("✅ update: updated reading id={}", r.id),
        Err(err) => eprintln!("❌ update: error updating reading id={}: {:?}", id, err),
    }

    result
}

/// DELETE a reading
pub async fn delete(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    if let Some(model) = electricity_reading::Entity::find_by_id(id).one(db).await? {
        let am: electricity_reading::ActiveModel = model.clone().into();
        let res = am.delete(db).await;

        match &res {
            Ok(_) => println!("✅ delete: deleted reading id={}", model.id),
            Err(err) => eprintln!("❌ delete: error deleting reading id={}: {:?}", id, err),
        }

        res.map(|_| Some(model))
    } else {
        println!("⚠️ delete: reading id={} not found", id);
        Ok(None)
    }
}
