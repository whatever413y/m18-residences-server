use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::services::r2_service::get_signed_url;
use crate::services::r2_service::R2Config;

/// GET /api/files/receipts/:tenant_name/:filename
pub async fn get_receipt_signed_url_handler(
    Path((tenant_name, filename)): Path<(String, String)>,
    Extension(r2): Extension<R2Config>,
) -> impl IntoResponse {
    let key = format!("receipts/{}/{}", tenant_name, filename);

    match get_signed_url(&r2, &key, 600).await {
        Ok(url) => (StatusCode::OK, Json(serde_json::json!({ "url": url }))),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to generate URL" })),
        )
    }
}

/// GET /api/files/payments/:filename
pub async fn get_payment_signed_url_handler(
    Path(filename): Path<String>,
    Extension(r2): Extension<R2Config>,
) -> impl IntoResponse {
    let key = format!("payments/{}.png", filename);

    match get_signed_url(&r2, &key, 600).await {
        Ok(url) => (StatusCode::OK, Json(serde_json::json!({ "url": url }))),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Failed to generate URL" })),
        ),
    }
}
