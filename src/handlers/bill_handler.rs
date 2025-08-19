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
    match bill_service::get_all_bills_with_details(&db).await {
        Ok(items) => Ok(Json(items)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /bills/:tenant_id/bill
pub async fn get_bill_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    match bill_service::get_tenant_bill_with_details(&db, tenant_id).await {
        Ok(Some(bill_model)) => Ok(Json(bill_model)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /tenants/:tenant_id/bills
pub async fn get_bills_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, StatusCode> {
    match bill_service::get_all_bills_for_tenant(&db, tenant_id).await {
        Ok(items) => Ok(Json(items)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Create bill handler
pub async fn create_bill_handler(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<BillPayload>,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    let input = BillInput {
        tenant_id: payload.tenant_id,
        reading_id: payload.reading_id,
        room_charges: payload.room_charges,
        electric_charges: payload.electric_charges,
        additional_charges: payload.additional_charges.unwrap_or_default(),
        receipt_url: None,
    };

    match bill_service::create_bill(&db, input).await {
        Ok(bill_with_details) => Ok(Json(bill_with_details)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Update bill
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
        Ok(updated_bill) => Ok(Json(updated_bill)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

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

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().map(|s| s.to_string());

        if let Some(_file_name) = file_name {
    if name == "receipt_file" {
        let tenant_name = get_tenant_by_id(&db, tenant_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.name)
            .unwrap_or_else(|| "unknown".to_string());

        // extract extension
        let ext = _file_name.rsplit('.').next().unwrap_or("bin");

        // detect MIME type
        let mime_type = MimeGuess::from_ext(ext)
            .first_or_octet_stream()
            .to_string();

        let bytes = match field.bytes().await {
            Ok(bytes) => {
                println!("✅ Read {} bytes", bytes.len());
                bytes
            }
            Err(e) => {
                println!("❌ Failed to read field bytes: {:?}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let key = format!(
            "receipts/{}/{}-r{}",
            tenant_name,
            Utc::now().timestamp(),
            reading_id,
        );

        receipt_url = Some(
            upload_file(&r2, bytes, &key, &mime_type)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );

        println!("✅ Uploaded receipt: {:?}", receipt_url);
        continue;
    }
}

        // Text fields
        let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let preview =
            String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "[non-utf8 data]".to_string());

        match name.as_str() {
            "tenant_id" => tenant_id = preview.parse().unwrap_or_default(),
            "reading_id" => reading_id = preview.parse().unwrap_or_default(),
            "room_charges" => room_charges = preview.parse().unwrap_or_default(),
            "electric_charges" => electric_charges = preview.parse().unwrap_or_default(),
            "additional_charges" => {
                additional_charges = serde_json::from_slice(&bytes).unwrap_or_default()
            }
            "receipt_url" => receipt_url = Some(preview),
            _ => println!("⚠️ Unknown field: {}", name),
        }
    }

    println!("➡️ Constructing BillInput");
    let input = BillInput {
        tenant_id,
        reading_id,
        room_charges,
        electric_charges,
        additional_charges,
        receipt_url,
    };
    println!("📦 BillInput prepared: {:?}", input);

    match bill_service::update_bill(&db, id, input).await {
        Ok(updated_bill) => {
            println!("✅ Bill updated successfully: {:?}", updated_bill);
            Ok(Json(updated_bill))
        }
        Err(e) => {
            println!("❌ Failed to update bill: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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
