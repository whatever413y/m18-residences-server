use crate::entities::electricity_reading;
use crate::repository::electricity_reading_repo;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};

fn value_or_zero(v: sea_orm::ActiveValue<i32>) -> i32 {
    if let sea_orm::ActiveValue::Set(x) = v { x } else { 0 }
}

pub fn calculate_consumption(prev: sea_orm::ActiveValue<i32>, curr: sea_orm::ActiveValue<i32>) -> i32 {
    value_or_zero(curr) - value_or_zero(prev)
}

/// GET all readings
pub async fn get_all_readings(db: &DatabaseConnection) -> Result<Vec<electricity_reading::Model>, DbErr> {
    let result = electricity_reading_repo::get_all(db).await;
    if let Ok(list) = &result {
        println!("✅ get_all_readings: fetched {} readings", list.len());
    } else if let Err(err) = &result {
        eprintln!("❌ get_all_readings: error: {:?}", err);
    }
    result
}

/// GET reading by ID
pub async fn get_reading_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<electricity_reading::Model>, DbErr> {
    let result = electricity_reading_repo::get_by_id(db, id).await;
    match &result {
        Ok(Some(r)) => println!("✅ get_reading_by_id: found id={}", r.id),
        Ok(None) => println!("⚠️ get_reading_by_id: id={} not found", id),
        Err(err) => eprintln!("❌ get_reading_by_id: error id={}: {:?}", id, err),
    }
    result
}

/// CREATE reading
pub async fn create_reading(
    db: &DatabaseConnection,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.consumption = Set(calculate_consumption(item.prev_reading.clone(), item.curr_reading.clone()));

    let result = electricity_reading_repo::create(db, item).await;

    if let Ok(ref r) = result {
        println!("✅ create_reading: created id={}", r.id);
    } else if let Err(err) = &result {
        eprintln!("❌ create_reading: error: {:?}", err);
    }

    result
}

/// UPDATE reading
pub async fn update_reading(
    db: &DatabaseConnection,
    id: i32,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.id = Set(id);
    item.consumption = Set(calculate_consumption(item.prev_reading.clone(), item.curr_reading.clone()));

    let result = electricity_reading_repo::update(db, item).await;

    if let Ok(ref r) = result {
        println!("✅ update_reading: updated id={}", r.id);
    } else if let Err(err) = &result {
        eprintln!("❌ update_reading: error id={}: {:?}", id, err);
    }

    result
}

/// DELETE reading
pub async fn delete_reading(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    let result = electricity_reading_repo::delete(db, id).await;

    match &result {
        Ok(Some(r)) => println!("✅ delete_reading: deleted id={}", r.id),
        Ok(None) => println!("⚠️ delete_reading: id={} not found", id),
        Err(err) => eprintln!("❌ delete_reading: error id={}: {:?}", id, err),
    }

    result
}
