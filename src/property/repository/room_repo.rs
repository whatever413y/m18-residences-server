use m18_residences_db::entities::room;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Order, QueryOrder, Set};

/// Every room by name: case-insensitively first (closest to the old Postgres
/// collation), then exactly, so the order is total.
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<room::Model>, DbErr> {
    room::Entity::find()
        .order_by(Expr::cust("\"room\".\"name\" COLLATE NOCASE"), Order::Asc)
        .order_by_asc(room::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    room::Entity::find_by_id(id).one(db).await
}

/// INSERT … RETURNING.
pub async fn create(
    db: &DatabaseConnection,
    item: room::ActiveModel,
) -> Result<room::Model, DbErr> {
    item.insert(db).await
}

/// UPDATE … RETURNING of the set columns; `DbErr::RecordNotUpdated` if `id` doesn't exist.
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    mut item: room::ActiveModel,
) -> Result<room::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

/// DELETE … RETURNING in one statement: the deleted room, or `None` if there was none.
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    room::Entity::delete_by_id(id).exec_with_returning(db).await
}
