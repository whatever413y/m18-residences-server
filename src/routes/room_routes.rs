use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::room_handler;

pub fn room_routes() -> Router {
    Router::new()
        .route("/", get(room_handler::get_rooms))
        .route("/:id", get(room_handler::get_room))
        .route("/", post(room_handler::create_room))
        .route("/:id", put(room_handler::update_room))
        .route("/:id", delete(room_handler::delete_room))
}
