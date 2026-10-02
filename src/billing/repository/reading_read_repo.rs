//! Read-only access to `electricity_reading` (owned by property).
use m18_residences_db::entities::electricity_reading;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use crate::billing::repository::MAX_IDS_PER_QUERY;

pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<electricity_reading::Model>, DbErr> {
    electricity_reading::Entity::find_by_id(id).one(db).await
}

/// The readings with these ids (in no particular order): one query per
/// [`MAX_IDS_PER_QUERY`] ids.
pub async fn get_by_ids(
    db: &DatabaseConnection,
    ids: &[i32],
) -> Result<Vec<electricity_reading::Model>, DbErr> {
    let mut readings = Vec::new();
    for chunk in ids.chunks(MAX_IDS_PER_QUERY) {
        let found = electricity_reading::Entity::find()
            .filter(electricity_reading::Column::Id.is_in(chunk.iter().copied()))
            .all(db)
            .await?;
        readings.extend(found);
    }
    Ok(readings)
}
