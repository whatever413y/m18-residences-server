use axum::{Router, routing::get};

use crate::{app::AppState, billing::handlers::file_handler::get_file};

/// `/api/files`: public, every request carries its own signed link.
pub fn file_routes() -> Router<AppState> {
    Router::new().route("/api/files/{*key}", get(get_file))
}
