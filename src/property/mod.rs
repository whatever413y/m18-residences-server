//! Property — the future m18-residences-property-api: rooms, tenants and
//! electricity readings.
//!
//! Layers, one file per resource each: `routes/` → `handlers/` (extractors,
//! permissions, input) → `services/` (rules, error messages, logging) →
//! `repository/` (SeaORM queries). Owns the `room`, `tenant` and
//! `electricity_reading` tables.
use axum::Router;
use m18_residences_shared_rs::auth::AuthUser;

use crate::app::AppState;

pub mod handlers;
pub mod repository;
pub mod routes;
pub mod services;

/// `/api/rooms`, `/api/tenants` and `/api/electricity-readings`. Every route
/// needs a valid token; the handlers check the role.
pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(crate::property::routes::room_routes::room_routes())
        .merge(crate::property::routes::tenant_routes::tenant_routes())
        .merge(crate::property::routes::electricity_reading_routes::electricity_reading_routes())
        .route_layer(axum::middleware::from_extractor_with_state::<
            AuthUser,
            AppState,
        >(state.clone()))
}
