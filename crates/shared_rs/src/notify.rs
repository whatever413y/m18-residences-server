//! Notices for the admin (e.g. "a tenant uploaded a payment"): a Discord
//! webhook on the Worker, switched off with `DISCORD_WEBHOOK_URL=off`, a
//! recorder in tests.
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// The notice could not be delivered (network, Discord's answer). Never contains the webhook URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyError(pub String);

#[async_trait::async_trait]
pub trait Notifier: Send + Sync {
    /// Sends `message` (plain text) to the admin: `Ok(false)` when notices are switched off.
    async fn notify(&self, message: &str) -> Result<bool, NotifyError>;
}

/// [`Notifier`] that sends nothing: `DISCORD_WEBHOOK_URL=off` (local development, e2e).
pub struct Disabled;

#[async_trait::async_trait]
impl Notifier for Disabled {
    async fn notify(&self, _message: &str) -> Result<bool, NotifyError> {
        Ok(false)
    }
}

/// [`Notifier`] for tests: records every message; [`FakeNotifier::set_failing`]
/// simulates Discord being unreachable.
#[derive(Default)]
pub struct FakeNotifier {
    sent: Mutex<Vec<String>>,
    failing: AtomicBool,
}

impl FakeNotifier {
    /// The messages sent so far, oldest first.
    pub fn sent(&self) -> Vec<String> {
        self.sent.lock().expect("notifier lock").clone()
    }

    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl Notifier for FakeNotifier {
    async fn notify(&self, message: &str) -> Result<bool, NotifyError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(NotifyError("simulated Discord outage".into()));
        }
        self.sent
            .lock()
            .expect("notifier lock")
            .push(message.to_string());
        Ok(true)
    }
}

/// Whether `url` looks like a Discord webhook URL (checked when the configuration is read).
pub fn is_discord_webhook(url: &str) -> bool {
    [
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
    ]
    .iter()
    .any(|prefix| url.len() > prefix.len() && url.starts_with(prefix))
}

/// [`Notifier`] posting to a Discord channel's webhook (`DISCORD_WEBHOOK_URL`,
/// a secret: whoever has it can post to the channel).
#[cfg(target_arch = "wasm32")]
pub struct DiscordWebhook {
    url: String,
}

#[cfg(target_arch = "wasm32")]
impl DiscordWebhook {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    async fn post(&self, message: &str) -> worker::Result<u16> {
        // No mentions are parsed, so text from a tenant (e.g. a name) can never ping anyone.
        let body = serde_json::json!({ "content": message, "allowed_mentions": { "parse": [] } });
        let headers = worker::Headers::new();
        headers.set("content-type", "application/json")?;
        let mut init = worker::RequestInit::new();
        init.with_method(worker::Method::Post)
            .with_headers(headers)
            .with_body(Some(worker::wasm_bindgen::JsValue::from_str(
                &body.to_string(),
            )));
        let request = worker::Request::new_with_init(&self.url, &init)?;
        let response = worker::Fetch::Request(request).send().await?;
        Ok(response.status_code())
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait]
impl Notifier for DiscordWebhook {
    async fn notify(&self, message: &str) -> Result<bool, NotifyError> {
        let status = worker::send::SendFuture::new(self.post(message))
            .await
            // Keep the webhook URL (a secret) out of the error, wherever it might appear.
            .map_err(|e| NotifyError(e.to_string().replace(&self.url, "<webhook>")))?;
        if (200..300).contains(&status) {
            Ok(true)
        } else {
            Err(NotifyError(format!("Discord answered {status}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_discord_webhook_urls() {
        assert!(is_discord_webhook(
            "https://discord.com/api/webhooks/123/abc"
        ));
        assert!(is_discord_webhook(
            "https://discordapp.com/api/webhooks/123/abc"
        ));
        assert!(!is_discord_webhook("https://discord.com/api/webhooks/"));
        assert!(!is_discord_webhook(
            "http://discord.com/api/webhooks/123/abc"
        ));
        assert!(!is_discord_webhook(
            "https://example.com/api/webhooks/123/abc"
        ));
        assert!(!is_discord_webhook("off"));
    }

    #[tokio::test]
    async fn the_fake_records_messages_and_can_fail() {
        let fake = FakeNotifier::default();
        assert!(fake.notify("one").await.unwrap());
        fake.set_failing(true);
        assert!(fake.notify("two").await.is_err());
        assert_eq!(fake.sent(), vec!["one".to_string()]);
    }
}
