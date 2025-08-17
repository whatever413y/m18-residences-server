mod handlers;
mod middleware;
mod routes;
mod entities;
mod repository;
mod services;

use axum::{response::Json, routing::get, Extension, Router};
use std::{net::SocketAddr, time::Duration};
use middleware::{cors::cors_layer, db};
use tokio::signal;

use crate::routes::room_routes::room_routes;
use crate::routes::tenant_routes::tenant_routes;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // Connect to database
    let db = match db::connect().await {
        Ok(conn) => {
            println!("✅ Database connected successfully");
            conn
        }
        Err(err) => {
            eprintln!("❌ Database connection failed: {}", err);
            std::process::exit(1);
        }
    };

    // Build app
    let app = Router::new()
        .nest("/api/rooms", room_routes())
        .nest("/api/tenants", tenant_routes())
        .nest("/api/electricity-readings", routes::electricity_reading_routes::electricity_reading_routes())
        .nest("/api/bills", routes::bill_routes::bill_routes())
        .route("/", get(|| async { "Rental Management API is up" }))
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))
        .layer(cors_layer())
        .layer(Extension(db));

    let addr = SocketAddr::from(([0, 0, 0, 0], std::env::var("PORT")
        .unwrap_or("3001".to_string())
        .parse()
        .unwrap()));

    println!("Server running on {}", addr);

    // Create a Handle for graceful shutdown
    let handle = axum_server::Handle::new();

    // Spawn a task to listen for Ctrl+C
    let graceful = handle.clone();
    tokio::spawn(async move {
    signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    println!("⚠️ Ctrl+C received, shutting down...");
    graceful.graceful_shutdown(Some(Duration::from_secs(5)));
});


    // Start server with the handle
    if let Err(err) = axum_server::bind(addr)
        .handle(handle)
        .serve(app.into_make_service())
        .await
    {
        eprintln!("Server error: {}", err);
    }

    println!("Server has shut down gracefully");
}
