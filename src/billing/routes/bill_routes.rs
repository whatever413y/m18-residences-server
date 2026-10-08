use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware::from_extractor_with_state,
    routing::{get, put},
};
use m18_residences_shared_rs::auth::AuthUser;

use crate::{
    app::AppState,
    billing::handlers::bill_handler::{
        UPLOAD_LIMIT_BYTES, clear_payment_handler, create_bill_handler, delete_bill,
        get_bill_by_tenant, get_bill_years, get_bills, get_bills_by_tenant,
        update_bill_json_handler, update_bill_multipart_handler, upload_payment_handler,
    },
};

/// `/api/bills`: logged-in users only; roles are checked per handler.
pub fn bill_routes(state: &AppState) -> Router<AppState> {
    let bills = Router::new()
        .route("/", get(get_bills).post(create_bill_handler))
        .route("/years", get(get_bill_years))
        .route("/{tenant_id}/bill", get(get_bill_by_tenant))
        .route("/{tenant_id}/bills", get(get_bills_by_tenant))
        .route("/{id}", put(update_bill_json_handler).delete(delete_bill))
        .route(
            "/{id}/upload",
            put(update_bill_multipart_handler).layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES)),
        )
        .route(
            "/{id}/payment",
            put(upload_payment_handler)
                .layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES))
                .delete(clear_payment_handler),
        )
        .route_layer(from_extractor_with_state::<AuthUser, AppState>(
            state.clone(),
        ));
    Router::new().nest("/api/bills", bills)
}
