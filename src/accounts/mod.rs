//! Accounts — the future m18-residences-accounts-api: issues JWTs (admin and
//! tenant login) and validates them.
pub mod handlers;
pub mod repository;
pub mod routes;
pub mod services;

use axum::Router;

use crate::app::AppState;

pub fn router(_state: &AppState) -> Router<AppState> {
    Router::new().nest(
        "/api/auth",
        crate::accounts::routes::auth_routes::auth_routes(),
    )
}
