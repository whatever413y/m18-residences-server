use axum::Router;
use axum::routing::get;

use crate::app::AppState;
use crate::property::handlers::room_handler::{
    create_room, delete_room, get_room, get_rooms, update_room,
};

pub fn room_routes() -> Router<AppState> {
    Router::new()
        .route("/api/rooms", get(get_rooms).post(create_room))
        .route(
            "/api/rooms/{id}",
            get(get_room).put(update_room).delete(delete_room),
        )
}
