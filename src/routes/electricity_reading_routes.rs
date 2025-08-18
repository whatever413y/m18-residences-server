use crate::handlers::electricity_reading_handler::{
    create_reading, delete_reading, get_reading, get_readings, update_reading,
};
use axum::Router;
use axum::routing::{delete, get, post, put};

pub fn electricity_reading_routes() -> Router {
    Router::new()
        .route("/", get(get_readings))
        .route("/:id", get(get_reading))
        .route("/", post(create_reading))
        .route("/:id", put(update_reading))
        .route("/:id", delete(delete_reading))
}
