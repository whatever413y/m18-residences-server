//! Billing — the future m18-residences-billing-api: bills, additional charges,
//! receipt uploads, and signed links to stored files (the only R2 user).
pub mod handlers;
pub mod repository;
pub mod routes;
pub mod services;

use axum::Router;

use crate::{
    app::AppState,
    billing::routes::{
        bill_routes::bill_routes, file_routes::file_routes, signed_url_routes::signed_url_routes,
    },
};

pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(bill_routes(state))
        .merge(signed_url_routes(state))
        .merge(file_routes())
}
