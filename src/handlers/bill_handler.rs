use crate::services::{
    bill_service::{self, AdditionalChargeInput, BillInput, BillWithChargesAndReading},
    r2_service::{R2Config, upload_file},
    tenant_service::get_tenant_by_id,
};
use axum::{
    Extension, Json,
    extract::{Path, multipart::Multipart},
    http::StatusCode,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use mime_guess::MimeGuess;

#[derive(Deserialize)]
pub struct BillPayload {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Option<Vec<AdditionalChargeInput>>,
    pub receipt_url: Option<String>,
}

/// GET /bills
pub async fn get_bills(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, StatusCode> {
    bill_service::get_all_bills_with_details(&db)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// GET /bills/:tenant_id/bill
pub async fn get_bill_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    match bill_service::get_tenant_bill_with_details(&db, tenant_id).await {
        Ok(Some(bill)) => Ok(Json(bill)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tenants/:tenant_id/bills
pub async fn get_bills_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, StatusCode> {
    bill_service::get_all_bills_for_tenant(&db, tenant_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// POST /bills
pub async fn create_bill_handler(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<BillPayload>,
) -> Result<(StatusCode, Json<BillWithChargesAndReading>), StatusCode> {
    let input = BillInput {
        tenant_id: payload.tenant_id,
        reading_id: payload.reading_id,
        room_charges: payload.room_charges,
        electric_charges: payload.electric_charges,
        additional_charges: payload.additional_charges.unwrap_or_default(),
        receipt_url: None,
    };

    bill_service::create_bill(&db, input)
        .await
        .map(|bill| (StatusCode::CREATED, Json(bill)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// PUT /bills/:id (JSON update)
pub async fn update_bill_json_handler(
    Extension(db): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(payload): Json<BillPayload>,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    let input = BillInput {
        tenant_id: payload.tenant_id,
        reading_id: payload.reading_id,
        room_charges: payload.room_charges,
        electric_charges: payload.electric_charges,
        additional_charges: payload.additional_charges.unwrap_or_default(),
        receipt_url: payload.receipt_url,
    };

    match bill_service::update_bill(&db, id, input).await {
        Ok(updated) => Ok(Json(updated)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// PUT /bills/:id (Multipart update with file upload)
pub async fn update_bill_multipart_handler(
    Extension(db): Extension<DatabaseConnection>,
    Extension(r2): Extension<R2Config>,
    Path(id): Path<i32>,
    mut multipart: Multipart,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    let mut tenant_id: i32 = 0;
    let mut reading_id: i32 = 0;
    let mut room_charges: i32 = 0;
    let mut electric_charges: i32 = 0;
    let mut additional_charges: Vec<AdditionalChargeInput> = vec![];
    let mut receipt_url: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        // Handle file upload
        if name == "receipt_file" {
            if let Some(fname) = file_name {
                let tenant_name = get_tenant_by_id(&db, tenant_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| t.name)
                    .unwrap_or_else(|| "unknown".to_string());

                let ext = fname.rsplit('.').next().unwrap_or("bin");
                let mime_type = MimeGuess::from_ext(ext).first_or_octet_stream().to_string();

                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                let key = format!("receipts/{}/{}-r{}", tenant_name, Utc::now().timestamp(), reading_id);

                receipt_url = Some(
                    upload_file(&r2, bytes, &key, &mime_type)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                );
            }
            continue;
        }

        // Handle text fields
        let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let value = String::from_utf8(bytes.to_vec()).unwrap_or_default();

        match name.as_str() {
            "tenant_id" => tenant_id = value.parse().unwrap_or_default(),
            "reading_id" => reading_id = value.parse().unwrap_or_default(),
            "room_charges" => room_charges = value.parse().unwrap_or_default(),
            "electric_charges" => electric_charges = value.parse().unwrap_or_default(),
            "additional_charges" => {
                additional_charges = serde_json::from_slice(&bytes).unwrap_or_default()
            }
            "receipt_url" => receipt_url = Some(value),
            _ => {}
        }
    }

    let input = BillInput {
        tenant_id,
        reading_id,
        room_charges,
        electric_charges,
        additional_charges,
        receipt_url,
    };

    match bill_service::update_bill(&db, id, input).await {
        Ok(bill) => Ok(Json(bill)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// DELETE /bills/:id
pub async fn delete_bill(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match bill_service::delete_bill_with_charges(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
