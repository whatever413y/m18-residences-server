use crate::entities::electricity_reading;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder,
    TransactionTrait,
};

/// GET all readings
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<electricity_reading::Model>, DbErr> {
    electricity_reading::Entity::find()
        .order_by_desc(electricity_reading::Column::CreatedAt)
        .all(db)
        .await
}

/// GET reading by ID
pub async fn get_by_id<C>(
    conn: &C,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr>
where
    C: ConnectionTrait + TransactionTrait,
{
    electricity_reading::Entity::find_by_id(id).one(conn).await
}

/// CREATE a new reading
pub async fn create(
    db: &DatabaseConnection,
    item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.insert(db).await
}

/// UPDATE a reading
pub async fn update(
    db: &DatabaseConnection,
    item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.update(db).await
}

/// DELETE a reading
pub async fn delete(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    if let Some(model) = electricity_reading::Entity::find_by_id(id).one(db).await? {
        let am: electricity_reading::ActiveModel = model.clone().into();
        am.delete(db).await.map(|_| Some(model))
    } else {
        Ok(None)
    }
}
