use m18_residences_server::app::app;
use m18_residences_server::middleware::db;
use m18_residences_server::services;
use std::{net::SocketAddr, time::Duration};

#[tokio::main]
async fn main() {
    // rustls is compiled with two crypto backends (ring via sqlx, aws-lc-rs via the AWS SDK), so it can't
    // choose a default on its own; TLS paths that rely on the default (e.g. sslmode=verify-ca) would panic.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    dotenvy::dotenv().ok();

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

    // Build app
    let router = app(db, r2);

    // Server address
    let addr = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT").unwrap_or("50000".to_string()).parse().unwrap(),
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
        .serve(router.into_make_service())
        .await
    {
        eprintln!("Server error: {}", err);
    }

    println!("Server has shut down gracefully");
}
