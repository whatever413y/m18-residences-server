pub mod electricity_reading_handler;
pub mod room_handler;
pub mod tenant_handler;

use m18_residences_shared_rs::error::ApiError;
use serde::Deserialize;

/// The `{id}` path parameter.
#[derive(Deserialize)]
pub struct IdParam {
    pub id: i32,
}

/// A name must have some non-whitespace text (it is stored as given).
pub fn require_name(name: &str) -> Result<(), ApiError> {
    if name.trim().is_empty() {
        Err(ApiError::BadRequest("name must not be empty".into()))
    } else {
        Ok(())
    }
}
