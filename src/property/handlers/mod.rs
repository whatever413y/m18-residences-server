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

/// The longest tenant name, in characters.
pub const TENANT_NAME_MAX_CHARS: usize = 64;

/// A tenant's name, trimmed: it is their login and part of their files' keys,
/// so it must be 1–64 characters without control characters, `/`, `\` or `..`.
pub fn tenant_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    require_name(name)?;
    if name.chars().count() > TENANT_NAME_MAX_CHARS {
        return Err(ApiError::BadRequest(format!(
            "name must be at most {TENANT_NAME_MAX_CHARS} characters"
        )));
    }
    if name.chars().any(char::is_control)
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
    {
        return Err(ApiError::BadRequest(
            "name must not contain '/', '\\', '..' or control characters".into(),
        ));
    }
    Ok(name.to_string())
}
