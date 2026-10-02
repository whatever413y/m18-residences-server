use m18_residences_db::{Db, entities::electricity_reading};
use m18_residences_shared_rs::{error::ApiError, log_ok};
use sea_orm::{ActiveValue::Set, DbErr};

use crate::property::repository::{electricity_reading_repo, room_repo, tenant_repo};
use crate::property::services::{Violation, violation};

fn not_found(id: i32) -> ApiError {
    ApiError::NotFound(format!("Reading {id} not found"))
}

/// consumption = curr - prev (negative when the meter went down, as before; 400 if it overflows).
pub fn calculate_consumption(prev: i32, curr: i32) -> Result<i32, ApiError> {
    curr.checked_sub(prev)
        .ok_or_else(|| ApiError::BadRequest("curr_reading - prev_reading is out of range".into()))
}

/// Sets `consumption` from the `prev_reading` and `curr_reading` the handler set.
fn with_consumption(
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::ActiveModel, ApiError> {
    let (Some(&prev), Some(&curr)) = (
        item.prev_reading.try_as_ref(),
        item.curr_reading.try_as_ref(),
    ) else {
        return Err(ApiError::Internal(
            "reading without prev_reading or curr_reading".into(),
        ));
    };
    item.consumption = Set(calculate_consumption(prev, curr)?);
    Ok(item)
}

/// A failed insert/update: a missing tenant or room is a 409 naming it; the
/// rest by the default mapping.
async fn write_error(db: &Db, err: DbErr, item: &electricity_reading::ActiveModel) -> ApiError {
    if violation(&err) != Some(Violation::ForeignKey) {
        return err.into();
    }
    if let Some(&tenant_id) = item.tenant_id.try_as_ref() {
        match tenant_repo::get_by_id(db.conn(), tenant_id).await {
            Ok(None) => return ApiError::Conflict(format!("Tenant {tenant_id} does not exist")),
            Ok(Some(_)) => {}
            Err(lookup) => return lookup.into(),
        }
    }
    if let Some(&room_id) = item.room_id.try_as_ref() {
        match room_repo::get_by_id(db.conn(), room_id).await {
            Ok(None) => return ApiError::Conflict(format!("Room {room_id} does not exist")),
            Ok(Some(_)) => {}
            Err(lookup) => return lookup.into(),
        }
    }
    err.into()
}

/// GET all readings, newest first.
pub async fn get_all_readings(db: &Db) -> Result<Vec<electricity_reading::Model>, ApiError> {
    let readings = electricity_reading_repo::get_all(db.conn()).await?;
    log_ok!("get_all_readings: fetched {} readings", readings.len());
    Ok(readings)
}

/// GET reading by ID (404 if missing).
pub async fn get_reading_by_id(db: &Db, id: i32) -> Result<electricity_reading::Model, ApiError> {
    let reading = electricity_reading_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    log_ok!("get_reading_by_id: found id={}", reading.id);
    Ok(reading)
}

/// CREATE reading with the computed consumption (409 if the tenant or room doesn't exist).
pub async fn create_reading(
    db: &Db,
    item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, ApiError> {
    let item = with_consumption(item)?;
    let reading = match electricity_reading_repo::create(db.conn(), item.clone()).await {
        Ok(reading) => reading,
        Err(err) => return Err(write_error(db, err, &item).await),
    };
    log_ok!("create_reading: created id={}", reading.id);
    Ok(reading)
}

/// UPDATE reading, recomputing consumption (404 if missing, 409 as for create).
/// `updated_at` is left as is, and a bill of this reading is not touched.
pub async fn update_reading(
    db: &Db,
    id: i32,
    mut item: electricity_reading::ActiveModel,
) -> Result<electricity_reading::Model, ApiError> {
    item.id = Set(id);
    let item = with_consumption(item)?;
    let reading = match electricity_reading_repo::update(db.conn(), item.clone()).await {
        Ok(reading) => reading,
        Err(DbErr::RecordNotUpdated) => return Err(not_found(id)),
        Err(err) => return Err(write_error(db, err, &item).await),
    };
    log_ok!("update_reading: updated id={}", reading.id);
    Ok(reading)
}

/// DELETE reading (404 if missing, 409 while a bill refers to it).
pub async fn delete_reading(db: &Db, id: i32) -> Result<electricity_reading::Model, ApiError> {
    let reading = electricity_reading_repo::delete(db.conn(), id)
        .await
        .map_err(|err| match violation(&err) {
            Some(Violation::ForeignKey) => {
                ApiError::Conflict(format!("Reading {id} still has a bill"))
            }
            _ => err.into(),
        })?
        .ok_or_else(|| not_found(id))?;
    log_ok!("delete_reading: deleted id={}", reading.id);
    Ok(reading)
}
