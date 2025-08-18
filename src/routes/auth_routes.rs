use axum::{
    routing::post,
    Router,
};
use crate::handlers::auth_handler::{
    admin_login_handler,
    tenant_login_handler,
};

pub fn auth_routes() -> Router {
    Router::new()
        .route("/admin-login", post(admin_login_handler))
        .route("/login", post(
            tenant_login_handler
        ))
}
