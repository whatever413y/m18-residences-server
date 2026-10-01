//! Billing — the future m18-residences-billing-api: bills, additional charges,
//! receipt uploads, and signed links to stored files (the only R2 user).
use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
