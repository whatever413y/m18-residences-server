use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use crate::entities::tenant;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .order_by_asc(tenant::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find_by_id(id).one(db).await
}

pub async fn get_by_name(db: &DatabaseConnection, name: &str) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .filter(tenant::Column::Name.eq(name))
        .one(db)
        .await
}

pub async fn create(db: &DatabaseConnection, item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    item.insert(db).await
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    if let Some(model) = tenant::Entity::find_by_id(id).one(db).await? {
        let am: tenant::ActiveModel = model.clone().into();
        am.delete(db).await.map(|_| Some(model))
    } else {
        Ok(None)
    }
}
