//! Read-only access to the `tenant` table (owned by the property domain).
use m18_residences_db::entities::tenant;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, QueryFilter, QuerySelect, sea_query::Expr};

/// Up to two tenants named `name` in any case (SQLite's NOCASE: ASCII letters).
/// Names are unique in any case (migration 0006), so two means the data predates it.
pub async fn find_by_name_ignoring_case(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Vec<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .filter(Expr::cust_with_values(
            "\"tenant\".\"name\" = ? COLLATE NOCASE",
            [name],
        ))
        .limit(2)
        .all(db)
        .await
}
