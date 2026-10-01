//! Property — the future m18-residences-property-api: rooms, tenants and
//! electricity readings.
use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new()
}
