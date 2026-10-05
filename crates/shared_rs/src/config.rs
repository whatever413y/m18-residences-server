//! Required configuration, from the Worker's vars and secrets (a map in tests).
//! Every problem is reported at once, and a misconfigured Worker answers every
//! request with an error instead of running with defaults.

use crate::cors::{AllowedOrigin, parse_origins};

#[derive(Clone, Debug)]
pub struct Config {
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String,
    /// Browser origins allowed to call the API (`ALLOWED_ORIGINS`, comma-separated).
    pub allowed_origins: Vec<AllowedOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(pub Vec<String>);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid configuration: {}", self.0.join("; "))
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut problems = Vec::new();
        let mut required = |name: &str| match get(name) {
            Some(value) if !value.trim().is_empty() => value,
            _ => {
                problems.push(format!("{name} is missing or empty"));
                String::new()
            }
        };
        let jwt_secret = required("JWT_SECRET");
        let admin_username = required("ADMIN_USERNAME");
        let admin_password = required("ADMIN_PASSWORD");
        let origins = required("ALLOWED_ORIGINS");

        let allowed_origins = if origins.is_empty() {
            Vec::new()
        } else {
            parse_origins(&origins).unwrap_or_else(|e| {
                problems.push(format!("ALLOWED_ORIGINS: {e}"));
                Vec::new()
            })
        };

        if problems.is_empty() {
            Ok(Self {
                jwt_secret,
                admin_username,
                admin_password,
                allowed_origins,
            })
        } else {
            Err(ConfigError(problems))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn reads_a_complete_configuration() {
        let config = Config::from_lookup(lookup(&[
            ("JWT_SECRET", "s3cret"),
            ("ADMIN_USERNAME", "admin"),
            ("ADMIN_PASSWORD", "pw"),
            (
                "ALLOWED_ORIGINS",
                "http://localhost:50001, http://localhost:50002",
            ),
        ]))
        .unwrap();
        assert_eq!(config.jwt_secret, "s3cret");
        assert_eq!(config.allowed_origins.len(), 2);
    }

    #[test]
    fn reports_every_problem_at_once() {
        let err = Config::from_lookup(lookup(&[
            ("JWT_SECRET", "  "),
            ("ADMIN_USERNAME", "admin"),
            ("ALLOWED_ORIGINS", "http://ok, not an origin"),
        ]))
        .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("JWT_SECRET is missing or empty"), "{text}");
        assert!(
            text.contains("ADMIN_PASSWORD is missing or empty"),
            "{text}"
        );
        assert!(text.contains("ALLOWED_ORIGINS"), "{text}");
        assert!(!text.contains("ADMIN_USERNAME"), "{text}");
    }
}
