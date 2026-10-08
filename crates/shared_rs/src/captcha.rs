//! The login captcha: Cloudflare Turnstile's token, checked with Cloudflare's
//! `siteverify` endpoint on the Worker, by a stand-in in tests.
use std::sync::atomic::{AtomicBool, Ordering};

/// Cloudflare could not be asked (network, bad answer): not "the token is bad".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptchaError(pub String);

#[async_trait::async_trait]
pub trait CaptchaVerifier: Send + Sync {
    /// Whether Cloudflare accepts `token` (single use) from the client at `ip`.
    async fn verify(&self, token: &str, ip: Option<&str>) -> Result<bool, CaptchaError>;
}

/// [`CaptchaVerifier`] for tests: accepts exactly [`FakeCaptcha::VALID_TOKEN`];
/// [`FakeCaptcha::set_unreachable`] simulates Cloudflare being down.
#[derive(Default)]
pub struct FakeCaptcha {
    unreachable: AtomicBool,
}

impl FakeCaptcha {
    pub const VALID_TOKEN: &str = "valid-turnstile-token";

    pub fn set_unreachable(&self, unreachable: bool) {
        self.unreachable.store(unreachable, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl CaptchaVerifier for FakeCaptcha {
    async fn verify(&self, token: &str, _ip: Option<&str>) -> Result<bool, CaptchaError> {
        if self.unreachable.load(Ordering::SeqCst) {
            return Err(CaptchaError("simulated siteverify outage".into()));
        }
        Ok(token == Self::VALID_TOKEN)
    }
}

/// [`CaptchaVerifier`] calling Turnstile's `siteverify` with the widget's secret
/// (`TURNSTILE_SECRET`; Cloudflare's test secrets work locally and in e2e).
#[cfg(target_arch = "wasm32")]
pub struct Turnstile {
    secret: String,
}

#[cfg(target_arch = "wasm32")]
impl Turnstile {
    const SITEVERIFY: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    async fn siteverify(&self, token: &str, ip: Option<&str>) -> worker::Result<serde_json::Value> {
        let mut body = serde_json::json!({ "secret": self.secret, "response": token });
        if let Some(ip) = ip {
            body["remoteip"] = ip.into();
        }
        let headers = worker::Headers::new();
        headers.set("content-type", "application/json")?;
        let mut init = worker::RequestInit::new();
        init.with_method(worker::Method::Post)
            .with_headers(headers)
            .with_body(Some(worker::wasm_bindgen::JsValue::from_str(
                &body.to_string(),
            )));
        let request = worker::Request::new_with_init(Self::SITEVERIFY, &init)?;
        let mut response = worker::Fetch::Request(request).send().await?;
        if response.status_code() != 200 {
            return Err(worker::Error::RustError(format!(
                "siteverify answered {}",
                response.status_code()
            )));
        }
        response.json().await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait]
impl CaptchaVerifier for Turnstile {
    async fn verify(&self, token: &str, ip: Option<&str>) -> Result<bool, CaptchaError> {
        let answer = worker::send::SendFuture::new(self.siteverify(token, ip))
            .await
            .map_err(|e| CaptchaError(e.to_string()))?;
        answer["success"]
            .as_bool()
            .ok_or_else(|| CaptchaError(format!("siteverify answered without `success`: {answer}")))
    }
}
