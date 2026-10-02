//! Read-only access to `tenant` (owned by property).
use m18_residences_db::entities::tenant;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find_by_id(id).one(db).await
}
