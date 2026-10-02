//! `/api/auth`: public; every route checks its own credentials.
use axum::{Router, routing::post};

use crate::accounts::handlers::auth_handler::{
    admin_login_handler, tenant_login_handler, validate_token_handler,
};
use crate::app::AppState;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/validate-token", post(validate_token_handler))
        .route("/admin-login", post(admin_login_handler))
        .route("/login", post(tenant_login_handler))
}
