use crate::handlers::room_handler::{create_room, delete_room, get_room, get_rooms, update_room};
use axum::Router;
use axum::routing::{delete, get, post, put};

pub fn room_routes() -> Router {
    Router::new()
        .route("/", get(get_rooms))
        .route("/:id", get(get_room))
        .route("/", post(create_room))
        .route("/:id", put(update_room))
        .route("/:id", delete(delete_room))
}
