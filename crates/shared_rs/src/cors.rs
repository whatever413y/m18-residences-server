//! CORS for the browser apps: only the configured origins may call the API.
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// One entry of `ALLOWED_ORIGINS`: an exact origin (`https://admin.example.dev`), or a pattern whose host starts
/// with `*-` (`https://*-admin.example.dev`), where `*` stands for one DNS label prefix (`[a-z0-9-]+`, no dots).
/// Patterns are for preview links such as `development-admin…` and `pr-12-admin…`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowedOrigin {
    Exact(HeaderValue),
    Prefixed {
        /// `https://` or `http://`.
        scheme: String,
        /// Everything after the `*`, starting with `-` (e.g. `-admin.example.dev`).
        rest: String,
    },
}

impl AllowedOrigin {
    pub fn matches(&self, origin: &HeaderValue) -> bool {
        match self {
            Self::Exact(exact) => exact == origin,
            Self::Prefixed { scheme, rest } => {
                let Ok(origin) = origin.to_str() else {
                    return false;
                };
                origin
                    .strip_prefix(scheme.as_str())
                    .and_then(|host| host.strip_suffix(rest.as_str()))
                    .is_some_and(|label| {
                        !label.is_empty()
                            && label
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    })
            }
        }
    }
}

/// Parses a comma-separated origin list (`http(s)://host[:port]`, optionally `http(s)://*-host`), skipping empty
/// entries.
pub fn parse_origins(list: &str) -> Result<Vec<AllowedOrigin>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            let invalid = || {
                format!("invalid origin {origin:?} (expected scheme://host[:port] or scheme://*-host[:port])")
            };
            let (scheme, host) = match origin.split_once("://") {
                Some(("http", host)) => ("http://", host),
                Some(("https", host)) => ("https://", host),
                _ => return Err(invalid()),
            };
            if host.is_empty() || host.contains('/') {
                return Err(invalid());
            }
            match host.strip_prefix('*') {
                Some(rest) if rest.starts_with('-') && rest.len() > 1 && !rest.contains('*') => {
                    rest.parse::<HeaderValue>().map_err(|_| invalid())?;
                    Ok(AllowedOrigin::Prefixed {
                        scheme: scheme.to_string(),
                        rest: rest.to_string(),
                    })
                }
                Some(_) => Err(invalid()),
                None if host.contains('*') => Err(invalid()),
                None => origin
                    .parse::<HeaderValue>()
                    .map(AllowedOrigin::Exact)
                    .map_err(|_| invalid()),
            }
        })
        .collect()
}

pub fn cors_layer(origins: &[AllowedOrigin]) -> CorsLayer {
    let origins = origins.to_vec();
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::predicate(move |origin, _| {
            origins.iter().any(|allowed| allowed.matches(origin))
        }))
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
                AllowedOrigin::Exact(HeaderValue::from_static("http://localhost:50001")),
                AllowedOrigin::Exact(HeaderValue::from_static(
                    "https://admin.m18-residences.workers.dev"
                ))
            ]
        );
    }

    #[test]
    fn skips_empty_entries() {
        assert!(parse_origins("").unwrap().is_empty());
        assert_eq!(
            parse_origins("http://localhost:50001,, ").unwrap(),
            vec![AllowedOrigin::Exact(HeaderValue::from_static(
                "http://localhost:50001"
            ))]
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

    fn allows(list: &str, origin: &str) -> bool {
        let origins = parse_origins(list).unwrap();
        let origin = HeaderValue::from_str(origin).unwrap();
        origins.iter().any(|o| o.matches(&origin))
    }

    #[test]
    fn a_prefix_pattern_matches_preview_hosts() {
        let list = "https://*-admin.m18-residences.workers.dev";
        assert!(allows(
            list,
            "https://development-admin.m18-residences.workers.dev"
        ));
        assert!(allows(
            list,
            "https://pr-12-admin.m18-residences.workers.dev"
        ));
        assert!(
            !allows(list, "https://admin.m18-residences.workers.dev"),
            "no label"
        );
        assert!(
            !allows(list, "https://a.b-admin.m18-residences.workers.dev"),
            "a dot in the label"
        );
        assert!(
            !allows(list, "https://Dev-admin.m18-residences.workers.dev"),
            "upper case"
        );
        assert!(
            !allows(list, "http://development-admin.m18-residences.workers.dev"),
            "other scheme"
        );
        assert!(!allows(
            list,
            "https://development-admin.m18-residences.workers.dev.evil.com"
        ));
        assert!(!allows(list, "https://evil.com"));
    }

    #[test]
    fn exact_origins_match_only_themselves() {
        let list = "https://admin.m18-residences.workers.dev";
        assert!(allows(list, "https://admin.m18-residences.workers.dev"));
        assert!(!allows(
            list,
            "https://development-admin.m18-residences.workers.dev"
        ));
    }

    #[test]
    fn rejects_invalid_patterns() {
        for bad in [
            "https://*.example.dev",
            "https://*",
            "https://*-",
            "https://dev-*.example.dev",
            "https://*-a*.example.dev",
            "*-admin.example.dev",
        ] {
            assert!(parse_origins(bad).is_err(), "{bad}");
        }
    }
}
