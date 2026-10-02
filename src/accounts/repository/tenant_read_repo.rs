//! Read-only access to the `tenant` table (owned by the property domain).
use m18_residences_db::entities::tenant;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

/// The tenant whose name is exactly `name` (case- and whitespace-sensitive).
pub async fn find_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .filter(tenant::Column::Name.eq(name))
        .one(db)
        .await
}
