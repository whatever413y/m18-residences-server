use crate::{
    entities::electricity_reading,
    repository::electricity_reading_repo,
};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};

fn value_or_zero(v: sea_orm::ActiveValue<i32>) -> i32 {
    if let sea_orm::ActiveValue::Set(x) = v { x } else { 0 }
}

pub fn calculate_consumption(prev: sea_orm::ActiveValue<i32>, curr: sea_orm::ActiveValue<i32>) -> i32 {
    value_or_zero(curr) - value_or_zero(prev)
}

/// CREATE reading 
pub async fn create_reading(
    db: &DatabaseConnection,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.consumption = Set(calculate_consumption(item.prev_reading.clone(), item.curr_reading.clone()));
    let now = Utc::now().naive_utc();
    item.created_at = Set(now);
    item.updated_at = Set(now);

    let result = electricity_reading_repo::create(db, item).await;

    if let Ok(ref r) = result {
        println!("✅ create_reading: created id={}", r.id);
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
    item.updated_at = Set(Utc::now().naive_utc());

    let result = electricity_reading_repo::update(db, item).await;

    if let Ok(ref r) = result {
        println!("✅ update_reading: updated id={}", r.id);
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
