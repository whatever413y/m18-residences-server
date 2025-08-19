use axum::http::{Method, header};
use tower_http::cors::{CorsLayer, AllowOrigin};

pub fn cors_layer() -> CorsLayer {
    let production_url = std::env::var("PRODUCTION_URL").unwrap_or_default();

    let mut origins = vec![
        "http://localhost:45794".parse().unwrap(),
    ];

    if !production_url.is_empty() {
        origins.push(production_url.parse().unwrap());
    }

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
}
