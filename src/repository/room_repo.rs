use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, DbErr, QueryOrder};
use crate::entities::room;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<room::Model>, DbErr> {
    room::Entity::find()
        .order_by_asc(room::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    room::Entity::find_by_id(id).one(db).await
}

pub async fn create(db: &DatabaseConnection, item: room::ActiveModel) -> Result<room::Model, DbErr> {
    item.insert(db).await
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: room::ActiveModel) -> Result<room::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    if let Some(model) = room::Entity::find_by_id(id).one(db).await? {
        let am: room::ActiveModel = model.clone().into();
        am.delete(db).await.map(|_| Some(model))
    } else {
        Ok(None)
    }
}
