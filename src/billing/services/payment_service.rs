//! Transitional: the payment-image API the apps used before payment methods (`GET /api/payments`,
//! `PUT /api/payments/{name}`, `GET /api/signed-urls/payments/{name}`), now backed by the `payment_method` table and
//! finding a method by the [`slug`] of its name. Remove once both apps with payment methods are live.
use m18_residences_db::Db;
use m18_residences_shared_rs::{error::ApiError, files::FileStore};
use serde::Serialize;

use crate::billing::services::payment_method_service::{self, slug};

#[derive(Debug, Serialize)]
pub struct PaymentImage {
    /// The method's slug, e.g. `gcash`.
    pub name: String,
    /// The storage key of its image (where it would be when it has none).
    pub key: String,
    /// Whether an image is stored for this method.
    pub exists: bool,
}

/// Every payment method with whether its image is stored.
pub async fn list(db: &Db, files: &dyn FileStore) -> Result<Vec<PaymentImage>, ApiError> {
    let methods = payment_method_service::list(db).await?;
    let mut images = Vec::with_capacity(methods.len());
    for method in methods {
        let name = slug(&method.name);
        let exists = match &method.image_key {
            Some(key) => files.head(key).await?.is_some(),
            None => false,
        };
        images.push(PaymentImage {
            key: method
                .image_key
                .unwrap_or_else(|| format!("payments/{name}.png")),
            name,
            exists,
        });
    }
    Ok(images)
}

/// Stores `bytes` (a PNG) as the image of the payment method named `name` (404 if there is none).
pub async fn replace(
    db: &Db,
    files: &dyn FileStore,
    name: &str,
    bytes: Vec<u8>,
) -> Result<PaymentImage, ApiError> {
    let method = payment_method_service::find_by_slug(db, name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Unknown payment method {name:?}")))?;
    let updated = payment_method_service::replace_image(db, files, method.id, bytes).await?;
    Ok(PaymentImage {
        name: name.into(),
        key: updated.image_key.unwrap_or_default(),
        exists: true,
    })
}

/// The storage key of the image of the method named `name` (404 if there is no such method or image).
pub async fn image_key(db: &Db, name: &str) -> Result<String, ApiError> {
    payment_method_service::find_by_slug(db, name)
        .await?
        .and_then(|m| m.image_key)
        .ok_or_else(|| ApiError::NotFound("Payment image not found".into()))
}
