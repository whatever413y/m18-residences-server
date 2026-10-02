use m18_residences_db::entities::tenant;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, Order, QueryFilter,
    QueryOrder, Set,
};

/// Every tenant (active or not) by name: case-insensitively first (closest to
/// the old Postgres collation), then exactly, so the order is total.
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .order_by(Expr::cust("\"tenant\".\"name\" COLLATE NOCASE"), Order::Asc)
        .order_by_asc(tenant::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find_by_id(id).one(db).await
}

/// Exact, case-sensitive match.
pub async fn get_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .filter(tenant::Column::Name.eq(name))
        .one(db)
        .await
}

/// INSERT … RETURNING.
pub async fn create(
    db: &DatabaseConnection,
    item: tenant::ActiveModel,
) -> Result<tenant::Model, DbErr> {
    item.insert(db).await
}

/// UPDATE … RETURNING of the set columns; `DbErr::RecordNotUpdated` if `id` doesn't exist.
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    mut item: tenant::ActiveModel,
) -> Result<tenant::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

/// DELETE … RETURNING in one statement: the deleted tenant, or `None` if there was none.
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::delete_by_id(id)
        .exec_with_returning(db)
        .await
}
