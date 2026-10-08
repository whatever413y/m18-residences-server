//! The Worker's entry points: requests (the state built from the bindings,
//! handed to the Axum router) and the daily scheduled run (retrying failed
//! archives).
use std::sync::Arc;

use axum::{body::Body, http::Response};
use m18_residences_shared_rs::{
    captcha::Turnstile, config::Config, log_error, r2::R2FileStore, rate_limit::CfRateLimiter,
};
use tower_service::Service;
use worker::{
    Context, Env, HttpRequest, ScheduleContext, ScheduledEvent, WorkerVersionMetadata, event,
};

use crate::app::{AppState, LoginGuards, app};
use crate::billing::services::file_cleanup_service;

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, _ctx: Context) -> worker::Result<Response<Body>> {
    let config = match Config::from_lookup(|name| lookup(&env, name)) {
        Ok(config) => config,
        Err(err) => {
            log_error!("{err}");
            return Ok(misconfigured());
        }
    };
    let db = m18_residences_db::d1::connect(env.d1("DB")?)
        .await
        .map_err(|e| worker::Error::RustError(format!("D1: {e}")))?;
    let files = Arc::new(R2FileStore::new(env.bucket("FILES")?));
    let login_guards = LoginGuards {
        limiter: Arc::new(CfRateLimiter::new(env.rate_limiter("LOGIN_RATE_LIMITER")?)),
        captcha: Arc::new(Turnstile::new(config.turnstile_secret.clone())),
    };
    let version = env
        .get_binding::<WorkerVersionMetadata>("CF_VERSION_METADATA")
        .ok()
        .map(|m| m.tag())
        .filter(|tag| !tag.is_empty());
    Ok(app(AppState::new(db, files, login_guards, config, version))
        .call(req)
        .await?)
}

/// A secret, or else a plain var.
fn lookup(env: &Env, name: &str) -> Option<String> {
    env.secret(name)
        .map(|s| s.to_string())
        .or_else(|_| env.var(name).map(|v| v.to_string()))
        .ok()
}

fn misconfigured() -> Response<Body> {
    Response::builder()
        .status(500)
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"error":"The server is misconfigured; see its logs"}"#,
        ))
        .expect("static response")
}

/// The daily run (`triggers.crons` in `wrangler.jsonc`): retries the archives that failed.
#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    if let Err(err) = retry_failed_archives(&env).await {
        log_error!("Scheduled file cleanup failed: {err}");
    }
}

async fn retry_failed_archives(env: &Env) -> Result<(), String> {
    let db = m18_residences_db::d1::connect(env.d1("DB").map_err(|e| format!("D1 binding: {e}"))?)
        .await
        .map_err(|e| format!("D1: {e}"))?;
    let files = R2FileStore::new(
        env.bucket("FILES")
            .map_err(|e| format!("R2 binding: {e}"))?,
    );
    file_cleanup_service::retry_archives(&db, &files)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
