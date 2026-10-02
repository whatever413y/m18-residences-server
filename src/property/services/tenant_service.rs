use m18_residences_db::{Db, entities::tenant};
use m18_residences_shared_rs::{error::ApiError, log_ok};
use sea_orm::DbErr;

use crate::property::repository::tenant_repo;
use crate::property::services::{Violation, violation};

fn not_found(id: i32) -> ApiError {
    ApiError::NotFound(format!("Tenant {id} not found"))
}

/// A failed insert/update: a taken name or a missing room is a 409 that says
/// so; the rest by the default mapping.
fn write_error(err: DbErr, item: &tenant::ActiveModel) -> ApiError {
    match violation(&err) {
        Some(Violation::Unique) => match item.name.try_as_ref() {
            Some(name) => ApiError::Conflict(format!("A tenant named \"{name}\" already exists")),
            None => err.into(),
        },
        Some(Violation::ForeignKey) => match item.room_id.try_as_ref() {
            Some(room_id) => ApiError::Conflict(format!("Room {room_id} does not exist")),
            None => err.into(),
        },
        None => err.into(),
    }
}

/// Get all tenants (active or not), by name.
pub async fn get_all_tenants(db: &Db) -> Result<Vec<tenant::Model>, ApiError> {
    let tenants = tenant_repo::get_all(db.conn()).await?;
    log_ok!("get_all_tenants: fetched {} tenants", tenants.len());
    Ok(tenants)
}

/// Get tenant by ID (404 if missing).
pub async fn get_tenant_by_id(db: &Db, id: i32) -> Result<tenant::Model, ApiError> {
    let tenant = tenant_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    log_ok!(
        "get_tenant_by_id: found tenant id={} name={}",
        tenant.id,
        tenant.name
    );
    Ok(tenant)
}

/// Get tenant by exact name (404 if none).
pub async fn get_tenant_by_name(db: &Db, name: &str) -> Result<tenant::Model, ApiError> {
    let tenant = tenant_repo::get_by_name(db.conn(), name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant \"{name}\" not found")))?;
    log_ok!(
        "get_tenant_by_name: found tenant id={} name={}",
        tenant.id,
        tenant.name
    );
    Ok(tenant)
}

/// Create tenant (409 if the name is taken or the room doesn't exist).
pub async fn create_tenant(db: &Db, item: tenant::ActiveModel) -> Result<tenant::Model, ApiError> {
    let tenant = tenant_repo::create(db.conn(), item.clone())
        .await
        .map_err(|err| write_error(err, &item))?;
    log_ok!(
        "create_tenant: created id={} name={}",
        tenant.id,
        tenant.name
    );
    Ok(tenant)
}

/// Update tenant (404 if missing, 409 as for create). `updated_at` is left as is.
pub async fn update_tenant(
    db: &Db,
    id: i32,
    item: tenant::ActiveModel,
) -> Result<tenant::Model, ApiError> {
    let tenant = tenant_repo::update(db.conn(), id, item.clone())
        .await
        .map_err(|err| match err {
            DbErr::RecordNotUpdated => not_found(id),
            err => write_error(err, &item),
        })?;
    log_ok!(
        "update_tenant: updated tenant id={} name={}",
        tenant.id,
        tenant.name
    );
    Ok(tenant)
}

/// Delete tenant (404 if missing, 409 while readings or bills refer to it).
pub async fn delete_tenant(db: &Db, id: i32) -> Result<tenant::Model, ApiError> {
    let tenant = tenant_repo::delete(db.conn(), id)
        .await
        .map_err(|err| match violation(&err) {
            Some(Violation::ForeignKey) => {
                ApiError::Conflict(format!("Tenant {id} still has readings or bills"))
            }
            _ => err.into(),
        })?
        .ok_or_else(|| not_found(id))?;
    log_ok!(
        "delete_tenant: deleted tenant id={} name={}",
        tenant.id,
        tenant.name
    );
    Ok(tenant)
}
