use axum::http::Method;
use tower_http::cors::{CorsLayer, AllowOrigin};

pub fn cors_layer() -> CorsLayer {
    let production_url = std::env::var("PRODUCTION_URL").unwrap_or_default();

    let allowed_origins = vec![
        "http://localhost:45794",
        production_url.as_str(),
    ];

    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    for origin in allowed_origins {
        if !origin.is_empty() {
            layer = layer.allow_origin(AllowOrigin::exact(origin.parse().unwrap()));
        }
    }

    layer.allow_credentials(true)
}
