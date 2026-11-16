//! Required configuration, from the Worker's vars and secrets (a map in tests).
//! Every problem is reported at once, and a misconfigured Worker answers every
//! request with an error instead of running with defaults.

use crate::cors::{AllowedOrigin, parse_origins};
use crate::notify::is_discord_webhook;

#[derive(Clone, Debug)]
pub struct Config {
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String,
    /// Browser origins allowed to call the API (`ALLOWED_ORIGINS`, comma-separated).
    pub allowed_origins: Vec<AllowedOrigin>,
    /// The Turnstile widget's secret, for checking the logins' captcha tokens.
    pub turnstile_secret: String,
    /// Where admin notices go (`DISCORD_WEBHOOK_URL`, a secret): a Discord webhook URL, or
    /// `None` when it is explicitly `off`.
    pub discord_webhook_url: Option<String>,
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
        let turnstile_secret = required("TURNSTILE_SECRET");
        let webhook = required("DISCORD_WEBHOOK_URL");

        let allowed_origins = if origins.is_empty() {
            Vec::new()
        } else {
            parse_origins(&origins).unwrap_or_else(|e| {
                problems.push(format!("ALLOWED_ORIGINS: {e}"));
                Vec::new()
            })
        };

        let discord_webhook_url = match webhook.trim() {
            "" => None, // Already reported as missing.
            "off" => None,
            url if is_discord_webhook(url) => Some(url.to_string()),
            // The value is a secret: never repeat it.
            _ => {
                problems.push(
                    "DISCORD_WEBHOOK_URL must be a Discord webhook URL (https://discord.com/api/webhooks/...) or off"
                        .into(),
                );
                None
            }
        };

        if problems.is_empty() {
            Ok(Self {
                jwt_secret,
                admin_username,
                admin_password,
                allowed_origins,
                turnstile_secret,
                discord_webhook_url,
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
            ("TURNSTILE_SECRET", "turnstile"),
            (
                "DISCORD_WEBHOOK_URL",
                "https://discord.com/api/webhooks/1/token",
            ),
        ]))
        .unwrap();
        assert_eq!(config.jwt_secret, "s3cret");
        assert_eq!(config.allowed_origins.len(), 2);
        assert_eq!(
            config.discord_webhook_url.as_deref(),
            Some("https://discord.com/api/webhooks/1/token")
        );
    }

    fn with_webhook(value: &str) -> Result<Config, ConfigError> {
        Config::from_lookup(lookup(&[
            ("JWT_SECRET", "s3cret"),
            ("ADMIN_USERNAME", "admin"),
            ("ADMIN_PASSWORD", "pw"),
            ("ALLOWED_ORIGINS", "http://localhost:50001"),
            ("TURNSTILE_SECRET", "turnstile"),
            ("DISCORD_WEBHOOK_URL", value),
        ]))
    }

    #[test]
    fn notices_can_be_switched_off_explicitly() {
        assert_eq!(with_webhook("off").unwrap().discord_webhook_url, None);
    }

    #[test]
    fn rejects_a_webhook_url_that_is_not_discord_without_repeating_it() {
        let text = with_webhook("https://example.com/hook?secret=xyz")
            .unwrap_err()
            .to_string();
        assert!(text.contains("DISCORD_WEBHOOK_URL must be"), "{text}");
        assert!(!text.contains("xyz"), "{text}");
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
        assert!(
            text.contains("TURNSTILE_SECRET is missing or empty"),
            "{text}"
        );
        assert!(
            text.contains("DISCORD_WEBHOOK_URL is missing or empty"),
            "{text}"
        );
        assert!(!text.contains("ADMIN_USERNAME"), "{text}");
    }
}
