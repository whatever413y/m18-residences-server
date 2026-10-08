//! Limits how often one client may try something (the logins): Cloudflare's
//! Rate Limiting binding on the Worker, a counter in memory in tests.
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// The limiter itself failed (not "over the limit").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitError(pub String);

#[async_trait::async_trait]
pub trait RateLimiter: Send + Sync {
    /// Counts one attempt for `key`: whether it is still within the limit.
    async fn allow(&self, key: &str) -> Result<bool, RateLimitError>;
}

/// [`RateLimiter`] for tests: `limit` attempts per key, ever (no window);
/// [`MemoryRateLimiter::set_failing`] simulates the binding failing.
pub struct MemoryRateLimiter {
    limit: u32,
    counts: Mutex<HashMap<String, u32>>,
    failing: AtomicBool,
}

impl MemoryRateLimiter {
    pub fn new(limit: u32) -> Self {
        Self {
            limit,
            counts: Mutex::default(),
            failing: AtomicBool::new(false),
        }
    }

    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl RateLimiter for MemoryRateLimiter {
    async fn allow(&self, key: &str) -> Result<bool, RateLimitError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(RateLimitError("simulated rate limiter failure".into()));
        }
        let mut counts = self.counts.lock().unwrap();
        let count = counts.entry(key.to_string()).or_default();
        *count += 1;
        Ok(*count <= self.limit)
    }
}

/// [`RateLimiter`] on a Workers Rate Limiting binding (its limit and period are
/// set in `wrangler.jsonc`). It counts per Cloudflare location, so it is a
/// loose filter, not an exact count.
#[cfg(target_arch = "wasm32")]
pub struct CfRateLimiter(worker::send::SendWrapper<worker::RateLimiter>);

#[cfg(target_arch = "wasm32")]
impl CfRateLimiter {
    pub fn new(binding: worker::RateLimiter) -> Self {
        Self(worker::send::SendWrapper::new(binding))
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait]
impl RateLimiter for CfRateLimiter {
    async fn allow(&self, key: &str) -> Result<bool, RateLimitError> {
        worker::send::SendFuture::new(self.0.limit(key.to_string()))
            .await
            .map(|outcome| outcome.success)
            .map_err(|e| RateLimitError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn counts_each_key_on_its_own() {
        let limiter = MemoryRateLimiter::new(2);
        assert_eq!(limiter.allow("a").await, Ok(true));
        assert_eq!(limiter.allow("a").await, Ok(true));
        assert_eq!(limiter.allow("a").await, Ok(false));
        assert_eq!(limiter.allow("b").await, Ok(true));
        limiter.set_failing(true);
        assert!(limiter.allow("b").await.is_err());
    }
}
