use axum::http::{HeaderValue, Method, header};
use tower_http::cors::{CorsLayer, AllowOrigin};

/// Browser origins allowed to call the API: every entry of the comma-separated
/// `LOCALHOST_URL` (one per local Flutter app) plus `PRODUCTION_URL`.
fn allowed_origins(localhost_urls: &str, production_url: &str) -> Vec<HeaderValue> {
    localhost_urls
        .split(',')
        .chain(std::iter::once(production_url))
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            origin
                .parse()
                .unwrap_or_else(|_| panic!("Invalid CORS origin: {origin:?}"))
        })
        .collect()
}

pub fn cors_layer() -> CorsLayer {
    let origins = allowed_origins(
        &std::env::var("LOCALHOST_URL").unwrap_or_default(),
        &std::env::var("PRODUCTION_URL").unwrap_or_default(),
    );

    if origins.is_empty() {
        eprintln!("⚠️ CORS: no allowed origins; set LOCALHOST_URL and/or PRODUCTION_URL");
    }

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
}

// ---------------------- INLINE TESTS ----------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_localhost_list_and_production_url() {
        let origins = allowed_origins(
            "http://localhost:50001, http://localhost:50002",
            "https://example.github.io",
        );
        assert_eq!(
            origins,
            vec![
                HeaderValue::from_static("http://localhost:50001"),
                HeaderValue::from_static("http://localhost:50002"),
                HeaderValue::from_static("https://example.github.io"),
            ]
        );
    }

    #[test]
    fn test_skips_empty_entries() {
        assert!(allowed_origins("", "").is_empty());
        assert_eq!(
            allowed_origins("http://localhost:50001,,", " "),
            vec![HeaderValue::from_static("http://localhost:50001")]
        );
    }

    #[test]
    #[should_panic(expected = "Invalid CORS origin")]
    fn test_panics_on_invalid_origin() {
        allowed_origins("http://local\nhost:50001", "");
    }
}
