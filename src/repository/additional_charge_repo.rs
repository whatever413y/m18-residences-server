use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, QueryFilter, QueryOrder};
use crate::entities::additional_charge;

#[allow(dead_code)]
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<additional_charge::Model>, DbErr> {
    additional_charge::Entity::find()
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(db)
        .await
}

#[allow(dead_code)]
pub async fn get_all_by_bill_id(db: &DatabaseConnection, bill_id: i32) -> Result<Vec<additional_charge::Model>, DbErr> {
    additional_charge::Entity::find()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(db)
        .await
}

#[allow(dead_code)]
pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<additional_charge::Model>, DbErr> {
    additional_charge::Entity::find_by_id(id).one(db).await
}

pub async fn create(db: &DatabaseTransaction, item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    item.insert(db).await
}

#[allow(dead_code)]
pub async fn update(db: &DatabaseConnection, item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    item.update(db).await
}

#[allow(dead_code)]
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::Id.eq(id))
        .exec(db)
        .await
        .map(|_| ())
}

pub async fn delete_many_by_bill_id(db: &DatabaseTransaction, bill_id: i32) -> Result<(), DbErr> {
    additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .exec(db)
        .await
        .map(|_| ())
}
