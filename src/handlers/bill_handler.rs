use axum::{Extension, Json, extract::Path, http::StatusCode};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use crate::entities::{bill, additional_charge};
use crate::services::bill_service::{self, AdditionalChargeInput, BillInput, BillWithCharges, BillWithChargesAndReading};
use crate::repository::bill_repo;

#[derive(Deserialize)]
pub struct BillPayload {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Option<Vec<AdditionalChargeInput>>,
    pub receipt_file: Option<Vec<u8>>, 
    pub receipt_mime: Option<String>, 
}

/// GET /bills
pub async fn get_bills(
    Extension(db): Extension<DatabaseConnection>
) -> Result<Json<Vec<BillWithCharges>>, StatusCode> {
    let items = bill_repo::get_all(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(items))
}


/// GET /bills/:id
pub async fn get_bill(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Json<BillWithCharges>, StatusCode> {
    match bill_repo::get_by_id(&db, id).await {
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
    let items = bill_service::get_bills_for_tenant(&db, tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(items))
}

// /// POST /bills
// pub async fn create_bill(
//     Extension(db): Extension<DatabaseConnection>,
//     Json(payload): Json<BillPayload>,
// ) -> Result<Json<(bill::Model, Vec<additional_charge::Model>)>, StatusCode> {
//     let input = BillInput {
//         tenant_id: payload.tenant_id,
//         reading_id: payload.reading_id,
//         room_charges: payload.room_charges,
//         electric_charges: payload.electric_charges,
//         additional_charges: payload.additional_charges.unwrap_or_default(),
//         receipt_url: None, // handle R2 upload later if needed
//     };

//     let bill = bill_repo::create(&db, input)
//         .await
//         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     // Fetch the newly created bill with its additional charges
//     let result = bill_repo::get_by_id(&db, bill.id)
//         .await
//         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
//         .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

//     Ok(Json(result))
// }

// /// PUT /bills/:id
// pub async fn update_bill(
//     Path(id): Path<i32>,
//     Extension(db): Extension<DatabaseConnection>,
//     Json(payload): Json<BillPayload>,
// ) -> Result<Json<(bill::Model, Vec<additional_charge::Model>)>, StatusCode> {
//     let input = BillInput {
//         tenant_id: payload.tenant_id,
//         reading_id: payload.reading_id,
//         room_charges: payload.room_charges,
//         electric_charges: payload.electric_charges,
//         additional_charges: payload.additional_charges.unwrap_or_default(),
//         receipt_url: None, // handle R2 upload later if needed
//     };

//     let updated_bill = bill_repo::update(&db, id, input)
//         .await
//         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     // Fetch updated bill with additional charges
//     let result = bill_repo::get_by_id(&db, updated_bill.id)
//         .await
//         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
//         .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

//     Ok(Json(result))
// }

/// DELETE /bills/:id
pub async fn delete_bill(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<StatusCode, StatusCode> {
    match bill_repo::delete(&db, id).await {
        Ok(Some(_)) => Ok(StatusCode::NO_CONTENT),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
