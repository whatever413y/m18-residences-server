use axum::middleware::from_fn;
use axum::Router;
use axum::routing::{get, post, put, delete};
use crate::handlers::room_handler::{
    get_rooms, get_room, create_room, update_room, delete_room
};
use crate::middleware::jwt::require_admin;

pub fn room_routes() -> Router {
    Router::new()
        .route("/", get(get_rooms))
        .route("/:id", get(get_room))
        .route("/", post(create_room).route_layer(from_fn(require_admin)))
        .route("/:id", put(update_room).route_layer(from_fn(require_admin)))
        .route("/:id", delete(delete_room).route_layer(from_fn(require_admin)))
}
