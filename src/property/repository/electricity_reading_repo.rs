use m18_residences_db::entities::electricity_reading;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder};

/// Every reading, newest first (`id` breaks ties: timestamps have second precision).
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<electricity_reading::Model>, DbErr> {
    electricity_reading::Entity::find()
        .order_by_desc(electricity_reading::Column::CreatedAt)
        .order_by_desc(electricity_reading::Column::Id)
        .all(db)
        .await
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    electricity_reading::Entity::find_by_id(id).one(db).await
}

/// INSERT … RETURNING. The caller sets `consumption`.
pub async fn create(
    db: &DatabaseConnection,
    item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.insert(db).await
}

/// UPDATE … RETURNING of the set columns (`item.id` must be set);
/// `DbErr::RecordNotUpdated` if the reading doesn't exist.
pub async fn update(
    db: &DatabaseConnection,
    item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, DbErr> {
    item.update(db).await
}

/// DELETE … RETURNING in one statement: the deleted reading, or `None` if there was none.
pub async fn delete(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    electricity_reading::Entity::delete_by_id(id)
        .exec_with_returning(db)
        .await
}
