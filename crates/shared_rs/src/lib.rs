//! What every M18 Residences service shares (the Rust counterpart of the
//! diagram's shared-py package): authentication and permissions, errors,
//! configuration, CORS, file storage with signed links, logging, and the
//! login guards (rate limiting and the captcha).
pub mod auth;
pub mod captcha;
pub mod config;
pub mod cors;
pub mod error;
pub mod extract;
pub mod files;
pub mod log;
pub mod rate_limit;

#[cfg(target_arch = "wasm32")]
pub mod r2;
