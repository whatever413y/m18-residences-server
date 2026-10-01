//! Accounts — the future m18-residences-accounts-api: issues JWTs (admin and
//! tenant login) and validates them.
use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
