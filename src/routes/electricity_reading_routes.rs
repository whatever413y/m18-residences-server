use axum::middleware::from_fn;
use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::electricity_reading_handler::{
    get_readings, get_reading, create_reading, update_reading, delete_reading
};
use crate::middleware::jwt::require_admin;

pub fn electricity_reading_routes() -> Router {
    Router::new()
        .route("/", get(get_readings))
        .route("/:id", get(get_reading))
        .route("/", post(create_reading)).route_layer(from_fn(require_admin))
        .route("/:id", put(update_reading)).route_layer(from_fn(require_admin))
        .route("/:id", delete(delete_reading)).route_layer(from_fn(require_admin))
}
