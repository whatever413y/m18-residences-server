//! The Worker's entry point: builds the state from the bindings and hands the
//! request to the Axum router.
use std::sync::Arc;

use axum::{body::Body, http::Response};
use m18_residences_shared_rs::{config::Config, log_error, r2::R2FileStore};
use tower_service::Service;
use worker::{Context, Env, HttpRequest, WorkerVersionMetadata, event};

use crate::app::{AppState, app};

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
    let version = env
        .get_binding::<WorkerVersionMetadata>("CF_VERSION_METADATA")
        .ok()
        .map(|m| m.tag())
        .filter(|tag| !tag.is_empty());
    Ok(app(AppState::new(db, files, config, version))
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
