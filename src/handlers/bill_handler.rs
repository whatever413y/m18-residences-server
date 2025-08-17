use crate::repository::bill_repo;
use crate::services::bill_service::{
    self, AdditionalChargeInput, BillInput, BillWithChargesAndReading
};
use axum::{Extension, Json, extract::Path, http::StatusCode};
use sea_orm::DatabaseConnection;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BillPayload {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Option<Vec<AdditionalChargeInput>>,
    // pub receipt_file: Option<Vec<u8>>,
    // pub receipt_mime: Option<String>,
}

/// GET /bills
pub async fn get_bills(
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, StatusCode> {
    match bill_service::get_all_bills_with_details(&db).await {
        Ok(items) => Ok(Json(items)),
        Err(err) => {
            eprintln!("[get_bills] Failed to fetch all bills: {:?}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /bills/:tenant_id/bill
pub async fn get_bill_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<BillWithChargesAndReading>, StatusCode> {
    match bill_service::get_tenant_bill_with_details(&db, tenant_id).await {
        Ok(Some(bill_model)) => Ok(Json(bill_model)),
        Ok(None) => {
            eprintln!("[get_bill_by_tenant] No bill found for tenant_id={}", tenant_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(err) => {
            eprintln!("[get_bill_by_tenant] Failed to fetch bill for tenant_id={} : {:?}", tenant_id, err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /tenants/:tenant_id/bills
pub async fn get_bills_by_tenant(
    Path(tenant_id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, StatusCode> {
    match bill_service::get_all_bills_for_tenant(&db, tenant_id).await {
        Ok(items) => Ok(Json(items)),
        Err(err) => {
            eprintln!("[get_bills_by_tenant] Failed to fetch bills for tenant_id={} : {:?}", tenant_id, err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
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

    match crate::services::bill_service::create_bill(&db, input).await {
        Ok(bill_with_details) => Ok(Json(bill_with_details)),
        Err(err) => {
            eprintln!(
                "[create_bill_handler] Failed to create bill for tenant_id={} reading_id={} : {:?}",
                payload.tenant_id, payload.reading_id, err
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Update bill handler
pub async fn update_bill_handler(
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
        receipt_url: None,
    };

    match crate::services::bill_service::update_bill(&db, id, input).await {
        Ok(updated_bill_with_details) => Ok(Json(updated_bill_with_details)),
        Err(err) => {
            eprintln!(
                "[update_bill_handler] Failed to update bill id={} for tenant_id={} : {:?}",
                id, payload.tenant_id, err
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /bills/:id
pub async fn delete_bill(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match bill_repo::delete(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => {
            eprintln!("[delete_bill] No bill found with id={}", id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(err) => {
            eprintln!("[delete_bill] Failed to delete bill id={} : {:?}", id, err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

