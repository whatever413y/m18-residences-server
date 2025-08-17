use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::electricity_reading_handler;

pub fn electricity_reading_routes() -> Router {
    Router::new()
        .route("/", get(electricity_reading_handler::get_readings))
        .route("/:id", get(electricity_reading_handler::get_reading))
        .route("/", post(electricity_reading_handler::create_reading))
        .route("/:id", put(electricity_reading_handler::update_reading))
        .route("/:id", delete(electricity_reading_handler::delete_reading))
}
