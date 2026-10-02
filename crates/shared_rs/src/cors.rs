//! CORS for the browser apps: only the configured origins may call the API.
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Parses a comma-separated origin list (`http(s)://host[:port]`), skipping empty entries.
pub fn parse_origins(list: &str) -> Result<Vec<HeaderValue>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            let valid_scheme = origin.starts_with("http://") || origin.starts_with("https://");
            let no_path = origin
                .split("://")
                .nth(1)
                .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'));
            match origin.parse::<HeaderValue>() {
                Ok(value) if valid_scheme && no_path => Ok(value),
                _ => Err(format!(
                    "invalid origin {origin:?} (expected scheme://host[:port])"
                )),
            }
        })
        .collect()
}

pub fn cors_layer(origins: &[HeaderValue]) -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::list(origins.iter().cloned()))
        .allow_credentials(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_list_of_origins() {
        assert_eq!(
            parse_origins("http://localhost:50001, https://admin.m18-residences.workers.dev")
                .unwrap(),
            vec![
                HeaderValue::from_static("http://localhost:50001"),
                HeaderValue::from_static("https://admin.m18-residences.workers.dev")
            ]
        );
    }

    #[test]
    fn skips_empty_entries() {
        assert!(parse_origins("").unwrap().is_empty());
        assert_eq!(
            parse_origins("http://localhost:50001,, ").unwrap(),
            vec![HeaderValue::from_static("http://localhost:50001")]
        );
    }

    #[test]
    fn rejects_invalid_origins() {
        assert!(parse_origins("http://local\nhost:50001").is_err());
        assert!(parse_origins("localhost:50001").is_err(), "no scheme");
        assert!(
            parse_origins("https://example.com/app").is_err(),
            "a path is not an origin"
        );
    }
}
