//! Payment methods (banks, e-wallets) the admin manages and tenants pay with: a name, optional account details and
//! an optional QR image (a PNG in file storage). Every upload gets a new key `payments/<id>-<unix ms>.png`, so a
//! rename never moves a file and no cache shows an old image; the image a method no longer points at (replaced,
//! removed, method deleted) is archived after the database write.
use chrono::Utc;
use m18_residences_db::{Db, entities::payment_method};
use m18_residences_shared_rs::{
    error::{ApiError, Violation, violation},
    files::{FileStore, sniff},
    log_error, log_ok, log_warn,
};
use sea_orm::{ActiveValue::NotSet, DbErr, Set};
use serde::{Deserialize, Serialize};

use crate::billing::repository::payment_method_repo;
use crate::billing::services::archive::archive_file;

/// QR images are stored as PNG (QR codes stay sharp); the admin app converts them.
pub const PAYMENT_IMAGE_TYPE: &str = "image/png";
const NAME_MAX: usize = 40;
const ACCOUNT_NAME_MAX: usize = 80;
const ACCOUNT_NUMBER_MAX: usize = 40;
const SORT_ORDER_MAX: i32 = 999;

/// The body of `POST /api/payment-methods` and `PUT /api/payment-methods/{id}`.
#[derive(Debug, Deserialize)]
pub struct PaymentMethodInput {
    pub name: String,
    #[serde(default)]
    pub account_name: Option<String>,
    #[serde(default)]
    pub account_number: Option<String>,
    /// Position in the list; when missing, a new method goes last and an edited one stays where it is.
    #[serde(default)]
    pub sort_order: Option<i32>,
}

/// The API shape of a payment method (the storage key stays on the server).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PaymentMethodOut {
    pub id: i32,
    pub name: String,
    pub account_name: Option<String>,
    pub account_number: Option<String>,
    pub sort_order: i32,
    pub has_image: bool,
}

impl From<payment_method::Model> for PaymentMethodOut {
    fn from(m: payment_method::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            account_name: m.account_name,
            account_number: m.account_number,
            sort_order: m.sort_order,
            has_image: m.image_key.is_some(),
        }
    }
}

/// `value` trimmed: at most `max` characters, no control characters.
fn text(field: &str, value: &str, max: usize) -> Result<String, ApiError> {
    let value = value.trim();
    if value.chars().count() > max {
        return Err(ApiError::BadRequest(format!(
            "{field} must be at most {max} characters"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field} must not contain control characters"
        )));
    }
    Ok(value.to_owned())
}

/// Like [`text`]; missing or blank is `None`.
fn optional_text(field: &str, value: Option<&str>, max: usize) -> Result<Option<String>, ApiError> {
    let value = text(field, value.unwrap_or_default(), max)?;
    Ok((!value.is_empty()).then_some(value))
}

impl PaymentMethodInput {
    /// The validated columns; `sort_order` is left unset when missing.
    fn into_active_model(self) -> Result<payment_method::ActiveModel, ApiError> {
        let name = text("name", &self.name, NAME_MAX)?;
        if name.is_empty() {
            return Err(ApiError::BadRequest("name must not be empty".into()));
        }
        let sort_order = match self.sort_order {
            Some(order) if !(0..=SORT_ORDER_MAX).contains(&order) => {
                return Err(ApiError::BadRequest(format!(
                    "sort_order must be between 0 and {SORT_ORDER_MAX}"
                )));
            }
            Some(order) => Set(order),
            None => NotSet,
        };
        Ok(payment_method::ActiveModel {
            name: Set(name),
            account_name: Set(optional_text(
                "account_name",
                self.account_name.as_deref(),
                ACCOUNT_NAME_MAX,
            )?),
            account_number: Set(optional_text(
                "account_number",
                self.account_number.as_deref(),
                ACCOUNT_NUMBER_MAX,
            )?),
            sort_order,
            ..Default::default()
        })
    }
}

/// The name in URLs and file names: lowercase, every run of other characters than `a-z`/`0-9` one `-`
/// ("GCash" → `gcash`, "Union Bank" → `union-bank`).
pub fn slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

fn not_found(id: i32) -> ApiError {
    ApiError::NotFound(format!("Payment method {id} not found"))
}

/// A failed insert/update: a taken name (any case) is a 409 that says so; the rest by the default mapping.
fn write_error(err: DbErr, item: &payment_method::ActiveModel) -> ApiError {
    match (violation(&err), item.name.try_as_ref()) {
        (Some(Violation::Unique), Some(name)) => {
            ApiError::Conflict(format!("A payment method named \"{name}\" already exists"))
        }
        _ => err.into(),
    }
}

/// Every payment method, in list order.
pub async fn list(db: &Db) -> Result<Vec<payment_method::Model>, ApiError> {
    Ok(payment_method_repo::get_all(db.conn()).await?)
}

/// One payment method (404 if missing).
pub async fn get(db: &Db, id: i32) -> Result<payment_method::Model, ApiError> {
    payment_method_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))
}

/// The method whose name has this [`slug`], if any.
pub async fn find_by_slug(
    db: &Db,
    wanted: &str,
) -> Result<Option<payment_method::Model>, ApiError> {
    Ok(list(db)
        .await?
        .into_iter()
        .find(|m| slug(&m.name) == wanted))
}

/// Adds a method without an image (400 if invalid, 409 if the name is taken); last in the list unless placed.
pub async fn create(db: &Db, input: PaymentMethodInput) -> Result<payment_method::Model, ApiError> {
    let mut item = input.into_active_model()?;
    if item.sort_order.is_not_set() {
        item.sort_order = Set(payment_method_repo::next_sort_order(db.conn()).await?);
    }
    let created = payment_method_repo::create(db.conn(), item.clone())
        .await
        .map_err(|err| write_error(err, &item))?;
    log_ok!(
        "Created payment method id={} name={}",
        created.id,
        created.name
    );
    Ok(created)
}

/// Replaces a method's name and account details, and its place when given (404 if missing, 409 if the name is
/// taken). The image is left as is.
pub async fn update(
    db: &Db,
    id: i32,
    input: PaymentMethodInput,
) -> Result<payment_method::Model, ApiError> {
    let item = input.into_active_model()?;
    let updated = payment_method_repo::update(db.conn(), id, item.clone())
        .await
        .map_err(|err| match err {
            DbErr::RecordNotUpdated => not_found(id),
            err => write_error(err, &item),
        })?;
    log_ok!(
        "Updated payment method id={} name={}",
        updated.id,
        updated.name
    );
    Ok(updated)
}

/// Deletes a method (404 if missing), then archives its image.
pub async fn delete(db: &Db, files: &dyn FileStore, id: i32) -> Result<(), ApiError> {
    let deleted = payment_method_repo::delete(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    log_ok!(
        "Deleted payment method id={} name={}",
        deleted.id,
        deleted.name
    );
    archive_image(files, deleted.id, deleted.image_key.as_deref()).await;
    Ok(())
}

/// Stores `bytes` (a PNG) as the method's QR image under a new key and points the method at it; the stored file is
/// removed again if that fails. Once it succeeds, the image it replaced is archived.
pub async fn replace_image(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
    bytes: Vec<u8>,
) -> Result<payment_method::Model, ApiError> {
    if sniff(&bytes) != Some(PAYMENT_IMAGE_TYPE) {
        return Err(ApiError::BadRequest(
            "Payment images must be PNG files".into(),
        ));
    }
    let old = get(db, id).await?;
    let key = format!("payments/{id}-{}.png", Utc::now().timestamp_millis());
    files.put(&key, bytes, PAYMENT_IMAGE_TYPE).await?;
    log_ok!("Stored payment image {key}");

    let updated = match payment_method_repo::set_image_key(db.conn(), id, Some(key.clone())).await {
        Ok(updated) => updated,
        Err(err) => {
            match files.delete(&key).await {
                Ok(()) => {
                    log_warn!("Removed payment image {key}: the payment method {id} update failed")
                }
                Err(delete_err) => log_error!(
                    "Payment image {key} is orphaned: the payment method {id} update failed and so did its removal ({})",
                    delete_err.0
                ),
            }
            return Err(match err {
                DbErr::RecordNotUpdated => not_found(id),
                err => err.into(),
            });
        }
    };
    if old.image_key.as_deref() != Some(key.as_str()) {
        archive_image(files, id, old.image_key.as_deref()).await;
    }
    Ok(updated)
}

/// Clears a method's QR image (404 if the method is missing), then archives the file.
pub async fn remove_image(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
) -> Result<payment_method::Model, ApiError> {
    let old = get(db, id).await?;
    let updated = payment_method_repo::set_image_key(db.conn(), id, None)
        .await
        .map_err(|err| match err {
            DbErr::RecordNotUpdated => not_found(id),
            err => err.into(),
        })?;
    archive_image(files, id, old.image_key.as_deref()).await;
    Ok(updated)
}

/// The storage key of a method's QR image (404 if the method is missing or has none).
pub async fn image_key(db: &Db, id: i32) -> Result<String, ApiError> {
    get(db, id)
        .await?
        .image_key
        .ok_or_else(|| ApiError::NotFound("This payment method has no QR image".into()))
}

/// Archives a QR image the method no longer points at. Called after the database write, so a failure leaves the
/// file at its key (logged), never a method pointing at a missing file.
async fn archive_image(files: &dyn FileStore, id: i32, key: Option<&str>) {
    let Some(key) = key else { return };
    match archive_file(files, key).await {
        Ok(()) => log_ok!("Archived payment image {key} of payment method {id}"),
        Err(err) => log_error!(
            "Payment image {key} of payment method {id} is orphaned: archiving it failed ({})",
            err.0
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slugs_are_lowercase_and_dashed() {
        assert_eq!(slug("GCash"), "gcash");
        assert_eq!(slug("BPI"), "bpi");
        assert_eq!(slug("  Union Bank (Savings) "), "union-bank-savings");
        assert_eq!(slug("Maya!"), "maya");
    }
}
