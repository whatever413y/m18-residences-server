use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use chrono::Utc;
use crate::entities::additional_charge;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<additional_charge::Model>, DbErr> {
    let result = additional_charge::Entity::find()
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(db)
        .await;

    match &result {
        Ok(list) => println!("✅ get_all: fetched {} additional charges", list.len()),
        Err(err) => eprintln!("❌ get_all: error fetching additional charges: {:?}", err),
    }

    result
}

pub async fn get_all_by_bill_id(db: &DatabaseConnection, bill_id: i32) -> Result<Vec<additional_charge::Model>, DbErr> {
    let result = additional_charge::Entity::find()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(db)
        .await;

    match &result {
        Ok(list) => println!("✅ get_all_by_bill_id: fetched {} additional charges for bill_id={}", list.len(), bill_id),
        Err(err) => eprintln!("❌ get_all_by_bill_id: error fetching additional charges for bill_id={}: {:?}", bill_id, err),
    }

    result
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<additional_charge::Model>, DbErr> {
    let result = additional_charge::Entity::find_by_id(id).one(db).await;

    match &result {
        Ok(Some(ac)) => println!("✅ get_by_id: found additional charge id={}", ac.id),
        Ok(None) => println!("⚠️ get_by_id: additional charge id={} not found", id),
        Err(err) => eprintln!("❌ get_by_id: error fetching additional charge id={}: {:?}", id, err),
    }

    result
}

pub async fn create(db: &DatabaseConnection, mut item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    let now = Utc::now().naive_utc();
    item.created_at = Set(now);
    item.updated_at = Set(now);

    let result = item.insert(db).await;

    match &result {
        Ok(ac) => println!("✅ create: created additional charge id={}", ac.id),
        Err(err) => eprintln!("❌ create: error creating additional charge: {:?}", err),
    }

    result
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    item.id = Set(id);
    item.updated_at = Set(Utc::now().naive_utc());

    let result = item.update(db).await;

    match &result {
        Ok(ac) => println!("✅ update: updated additional charge id={}", ac.id),
        Err(err) => eprintln!("❌ update: error updating additional charge id={}: {:?}", id, err),
    }

    result
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<additional_charge::Model>, DbErr> {
    if let Some(model) = additional_charge::Entity::find_by_id(id).one(db).await? {
        let am: additional_charge::ActiveModel = model.clone().into();
        let res = am.delete(db).await;

        match &res {
            Ok(_) => println!("✅ delete: deleted additional charge id={}", model.id),
            Err(err) => eprintln!("❌ delete: error deleting additional charge id={}: {:?}", id, err),
        }

        res.map(|_| Some(model))
    } else {
        println!("⚠️ delete: additional charge id={} not found", id);
        Ok(None)
    }
}

pub async fn delete_many_by_bill_id(db: &DatabaseConnection, bill_id: i32) -> Result<(), DbErr> {
    let result = additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .exec(db)
        .await;

    match &result {
        Ok(res) => println!("✅ delete_many_by_bill_id: deleted {} additional charges for bill_id={}", res.rows_affected, bill_id),
        Err(err) => eprintln!("❌ delete_many_by_bill_id: error deleting additional charges for bill_id={}: {:?}", bill_id, err),
    }

    result.map(|_| ())
}
