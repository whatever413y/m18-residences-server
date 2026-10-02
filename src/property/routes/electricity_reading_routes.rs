use axum::Router;
use axum::routing::get;

use crate::app::AppState;
use crate::property::handlers::electricity_reading_handler::{
    create_reading, delete_reading, get_reading, get_readings, update_reading,
};

pub fn electricity_reading_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/electricity-readings",
            get(get_readings).post(create_reading),
        )
        .route(
            "/api/electricity-readings/{id}",
            get(get_reading).put(update_reading).delete(delete_reading),
        )
}
