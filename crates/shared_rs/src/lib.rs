//! What every M18 Residences service shares (the Rust counterpart of the
//! diagram's shared-py package): authentication and permissions, errors,
//! configuration, CORS, file storage with signed links, and logging.
pub mod auth;
pub mod config;
pub mod cors;
pub mod error;
pub mod files;
pub mod log;

#[cfg(target_arch = "wasm32")]
pub mod r2;
