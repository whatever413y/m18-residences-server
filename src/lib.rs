//! M18 Residences API, deployed as one Cloudflare Worker.
//!
//! Each domain folder is a future service of its own. A domain only uses
//! itself, `crate::app`, and the shared crates (`m18_residences_db`,
//! `m18_residences_shared_rs`); `tests/boundaries.rs` enforces it. Extracting
//! one means moving its folder and adding an entry point.
pub mod accounts;
pub mod app;
pub mod billing;
pub mod property;

#[cfg(target_arch = "wasm32")]
mod worker_entry;
