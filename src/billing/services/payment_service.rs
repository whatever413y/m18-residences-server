//! Payment QR images (`payments/<name>.png`), one per fixed payment method;
//! the admin replaces them, everyone logged in views them via signed links.
use m18_residences_shared_rs::{
    error::ApiError,
    files::{FileStore, sniff},
    log_ok,
};
use serde::Serialize;

/// The payment methods the apps offer; their images are the only files under `payments/`.
pub const PAYMENT_METHODS: [&str; 3] = ["bpi", "gcash", "maya"];

/// Payment images are stored as PNG (QR codes stay sharp); the admin app converts them.
const PAYMENT_TYPE: &str = "image/png";

#[derive(Debug, Serialize)]
pub struct PaymentImage {
    pub name: String,
    /// The storage key, e.g. `payments/gcash.png`.
    pub key: String,
    /// Whether an image is stored for this method.
    pub exists: bool,
}

fn key(name: &str) -> String {
    format!("payments/{name}.png")
}

fn check_method(name: &str) -> Result<(), ApiError> {
    if PAYMENT_METHODS.contains(&name) {
        Ok(())
    } else {
        Err(ApiError::BadRequest(format!(
            "Unknown payment method {name:?}: expected one of {}",
            PAYMENT_METHODS.join(", ")
        )))
    }
}

/// Every payment method with whether its image is stored.
pub async fn list(files: &dyn FileStore) -> Result<Vec<PaymentImage>, ApiError> {
    let mut images = Vec::with_capacity(PAYMENT_METHODS.len());
    for name in PAYMENT_METHODS {
        let key = key(name);
        let exists = files.head(&key).await?.is_some();
        images.push(PaymentImage {
            name: name.into(),
            key,
            exists,
        });
    }
    Ok(images)
}

/// Stores `bytes` (a PNG) as the image of the payment method `name`, replacing the old one.
pub async fn replace(
    files: &dyn FileStore,
    name: &str,
    bytes: Vec<u8>,
) -> Result<PaymentImage, ApiError> {
    check_method(name)?;
    if sniff(&bytes) != Some(PAYMENT_TYPE) {
        return Err(ApiError::BadRequest(
            "Payment images must be PNG files".into(),
        ));
    }
    let key = key(name);
    files.put(&key, bytes, PAYMENT_TYPE).await?;
    log_ok!("Stored payment image {key}");
    Ok(PaymentImage {
        name: name.into(),
        key,
        exists: true,
    })
}
