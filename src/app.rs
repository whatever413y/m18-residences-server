//! Builds the API: the three domain routers, the health routes, CORS, and the
//! state every handler can reach.
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRef, State},
    routing::get,
};
use m18_residences_db::Db;
use m18_residences_shared_rs::{
    auth::JwtKeys,
    config::Config,
    cors::cors_layer,
    files::{FileSigner, FileStore},
};
use serde_json::{Value, json};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub files: Arc<dyn FileStore>,
    pub config: Arc<Config>,
    pub jwt: JwtKeys,
    pub signer: Arc<FileSigner>,
    /// The deployed version (the commit it was deployed with), if known.
    pub version: Option<String>,
}

impl AppState {
    pub fn new(db: Db, files: Arc<dyn FileStore>, config: Config, version: Option<String>) -> Self {
        let jwt = JwtKeys::new(&config.jwt_secret);
        let signer = Arc::new(FileSigner::new(&config.jwt_secret));
        Self {
            db,
            files,
            config: Arc::new(config),
            jwt,
            signer,
            version,
        }
    }
}

impl FromRef<AppState> for JwtKeys {
    fn from_ref(state: &AppState) -> Self {
        state.jwt.clone()
    }
}

pub fn app(state: AppState) -> Router {
    let cors = cors_layer(&state.config.allowed_origins);
    Router::new()
        .route("/", get(|| async { "API is up" }))
        .route("/health", get(health))
        .merge(crate::accounts::router(&state))
        .merge(crate::property::router(&state))
        .merge(crate::billing::router(&state))
        .layer(cors)
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "status": "ok", "version": state.version }))
}
