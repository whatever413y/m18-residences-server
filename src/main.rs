use m18_residences_server::services;
use m18_residences_server::routes;
use axum::{middleware::from_fn, response::Json, routing::get, Extension, Router};
use m18_residences_server::middleware::{cors::cors_layer, db, jwt::require_auth};
use std::{net::SocketAddr, time::Duration};

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

    // Initialize R2 client
    let r2 = services::r2_service::init_r2().await;

    // Helper to apply JWT auth to a router
    let protected = |router: Router| router.route_layer(from_fn(require_auth));

    // Build app
    let app = Router::new()
        // Public routes
        .nest("/api/auth", routes::auth_routes::auth_routes())
        .route("/", get(|| async { "API is up" }))
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))
        
        // Protected routes
        .nest("/api/signed-urls", protected(routes::signed_url_routes::signed_url_routes()))
        .nest("/api/rooms", protected(routes::room_routes::room_routes()))
        .nest("/api/tenants", protected(routes::tenant_routes::tenant_routes()))
        .nest(
            "/api/electricity-readings",
            protected(routes::electricity_reading_routes::electricity_reading_routes()),
        )
        .nest("/api/bills", protected(routes::bill_routes::bill_routes()))

        // Global layers
        .layer(cors_layer())
        .layer(Extension(db))
        .layer(Extension(r2));

    // Server address
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT").unwrap_or("3001".to_string()).parse().unwrap(),
    ));
    println!("Server running on {}", addr);

    // Create handle for graceful shutdown
    let handle = axum_server::Handle::new();
    let graceful = handle.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("⚠️ Ctrl+C received, shutting down...");
        graceful.graceful_shutdown(Some(Duration::from_secs(5)));
    });

    // Start server
    if let Err(err) = axum_server::bind(addr)
        .handle(handle)
        .serve(app.into_make_service())
        .await
    {
        eprintln!("Server error: {}", err);
    }

    println!("Server has shut down gracefully");
}
