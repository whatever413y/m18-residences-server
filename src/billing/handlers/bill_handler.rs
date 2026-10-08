use std::collections::HashMap;

use axum::{
    Json,
    extract::{
        State,
        multipart::{Multipart, MultipartError, MultipartRejection},
    },
    http::StatusCode,
};
use chrono::NaiveDate;
use m18_residences_shared_rs::{
    auth::{Admin, AuthUser},
    error::ApiError,
    extract::{ValidJson, ValidPath, ValidQuery},
};
use serde::Deserialize;

use crate::{
    app::AppState,
    billing::{
        repository::bill_repo::BillFilter,
        services::bill_service::{
            self, AdditionalChargeInput, BillInput, BillWithChargesAndReading, PaymentUpload,
            ReceiptUpload,
        },
    },
};

/// The largest multipart body `PUT /api/bills/{id}/upload` and `/payment` accept.
pub const UPLOAD_LIMIT_BYTES: usize = 10 * 1024 * 1024;

#[derive(Deserialize)]
pub struct BillPayload {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Option<Vec<AdditionalChargeInput>>,
    pub receipt_url: Option<String>,
}

impl BillPayload {
    fn into_input(self) -> BillInput {
        BillInput {
            tenant_id: self.tenant_id,
            reading_id: self.reading_id,
            room_charges: self.room_charges,
            electric_charges: self.electric_charges,
            // Absent or null means "no charges" (the documented contract), not a fallback for bad input.
            additional_charges: self.additional_charges.unwrap_or_default(),
            receipt_url: self.receipt_url,
        }
    }
}

#[derive(Deserialize)]
pub struct BillId {
    pub id: i32,
}

#[derive(Deserialize)]
pub struct TenantId {
    pub tenant_id: i32,
}

/// `GET /api/bills` filters; all optional, every one given must match.
#[derive(Deserialize)]
pub struct BillQuery {
    /// `YYYY-MM-DD`: bills created on or after it, plus every bill without a receipt.
    pub since: Option<NaiveDate>,
    /// Bills created in this year.
    pub year: Option<i32>,
    pub tenant_id: Option<i32>,
    /// Bills whose reading is in this room.
    pub room_id: Option<i32>,
}

/// GET /api/bills (admin): every bill, or those matching the query's filters.
pub async fn get_bills(
    State(state): State<AppState>,
    _admin: Admin,
    ValidQuery(query): ValidQuery<BillQuery>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, ApiError> {
    let filter = BillFilter {
        since: query.since,
        year: query.year,
        tenant_id: query.tenant_id,
        room_id: query.room_id,
    };
    Ok(Json(
        bill_service::get_bills_with_details(&state.db, &filter).await?,
    ))
}

/// GET /api/bills/years (admin): the years bills were created in, newest first.
pub async fn get_bill_years(
    State(state): State<AppState>,
    _admin: Admin,
) -> Result<Json<Vec<i32>>, ApiError> {
    Ok(Json(bill_service::get_bill_years(&state.db).await?))
}

/// GET /api/bills/{tenant_id}/bill (admin or that tenant)
pub async fn get_bill_by_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    path: ValidPath<TenantId>,
) -> Result<Json<BillWithChargesAndReading>, ApiError> {
    let ValidPath(TenantId { tenant_id }) = path;
    claims.ensure_admin_or_tenant(tenant_id)?;
    bill_service::get_tenant_bill_with_details(&state.db, tenant_id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("No bill found for tenant {tenant_id}")))
}

/// GET /api/bills/{tenant_id}/bills (admin or that tenant)
pub async fn get_bills_by_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    path: ValidPath<TenantId>,
) -> Result<Json<Vec<BillWithChargesAndReading>>, ApiError> {
    let ValidPath(TenantId { tenant_id }) = path;
    claims.ensure_admin_or_tenant(tenant_id)?;
    Ok(Json(
        bill_service::get_all_bills_for_tenant(&state.db, tenant_id).await?,
    ))
}

/// POST /api/bills (admin)
pub async fn create_bill_handler(
    State(state): State<AppState>,
    _admin: Admin,
    body: ValidJson<BillPayload>,
) -> Result<(StatusCode, Json<BillWithChargesAndReading>), ApiError> {
    let ValidJson(payload) = body;
    let created = bill_service::create_bill(&state.db, payload.into_input()).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// PUT /api/bills/{id} (admin; JSON update)
pub async fn update_bill_json_handler(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<BillId>,
    body: ValidJson<BillPayload>,
) -> Result<Json<BillWithChargesAndReading>, ApiError> {
    let ValidPath(BillId { id }) = path;
    let ValidJson(payload) = body;
    Ok(Json(
        bill_service::update_bill(&state.db, state.files.as_ref(), id, payload.into_input())
            .await?,
    ))
}

fn multipart_error(err: MultipartError) -> ApiError {
    if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::PayloadTooLarge(format!(
            "The upload is larger than {} MiB",
            UPLOAD_LIMIT_BYTES / (1024 * 1024)
        ))
    } else {
        ApiError::BadRequest(err.body_text())
    }
}

const NUMBER_FIELDS: [&str; 4] = [
    "tenant_id",
    "reading_id",
    "room_charges",
    "electric_charges",
];

/// The parts of an upload form, read completely before anything is validated.
#[derive(Default)]
struct UploadForm {
    text: HashMap<String, String>,
    /// The `receipt_file` part: its filename (if any) and bytes.
    file: Option<(Option<String>, Vec<u8>)>,
}

impl UploadForm {
    async fn read(mut multipart: Multipart) -> Result<Self, ApiError> {
        let mut form = Self::default();
        while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
            let Some(name) = field.name().map(str::to_owned) else {
                continue;
            };
            if name == "receipt_file" {
                let filename = field.file_name().map(str::to_owned);
                let bytes = field.bytes().await.map_err(multipart_error)?;
                form.file = Some((filename, bytes.to_vec()));
            } else if NUMBER_FIELDS.contains(&name.as_str())
                || name == "additional_charges"
                || name == "receipt_url"
            {
                let bytes = field.bytes().await.map_err(multipart_error)?;
                let value = String::from_utf8(bytes.to_vec())
                    .map_err(|_| ApiError::BadRequest(format!("{name} must be UTF-8 text")))?;
                form.text.insert(name, value);
            }
            // Other parts are ignored.
        }
        Ok(form)
    }

    fn number(&self, field: &str) -> Result<i32, ApiError> {
        let value = self
            .text
            .get(field)
            .ok_or_else(|| ApiError::BadRequest(format!("{field} is required")))?;
        value
            .trim()
            .parse()
            .map_err(|_| ApiError::BadRequest(format!("{field} must be an integer, got {value:?}")))
    }

    fn additional_charges(&self) -> Result<Vec<AdditionalChargeInput>, ApiError> {
        let Some(value) = self.text.get("additional_charges") else {
            return Ok(Vec::new());
        };
        serde_json::from_str::<Option<Vec<AdditionalChargeInput>>>(value)
            // `null` means no charges, as in the JSON body.
            .map(Option::unwrap_or_default)
            .map_err(|e| {
                ApiError::BadRequest(format!(
                    "additional_charges must be a JSON array of {{amount, description}}: {e}"
                ))
            })
    }

    fn into_input(self) -> Result<(BillInput, Option<ReceiptUpload>), ApiError> {
        let input = BillInput {
            tenant_id: self.number("tenant_id")?,
            reading_id: self.number("reading_id")?,
            room_charges: self.number("room_charges")?,
            electric_charges: self.number("electric_charges")?,
            additional_charges: self.additional_charges()?,
            receipt_url: self.text.get("receipt_url").cloned(),
        };
        let receipt = match self.file {
            None => None,
            Some((None, _)) => {
                return Err(ApiError::BadRequest(
                    "receipt_file must be a file part with a filename".into(),
                ));
            }
            Some((Some(_), bytes)) => Some(ReceiptUpload { bytes }),
        };
        Ok((input, receipt))
    }
}

/// PUT /api/bills/{id}/upload (admin; multipart update with an optional receipt file)
pub async fn update_bill_multipart_handler(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<BillId>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<BillWithChargesAndReading>, ApiError> {
    let ValidPath(BillId { id }) = path;
    let multipart = multipart.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let (input, receipt) = UploadForm::read(multipart).await?.into_input()?;
    Ok(Json(
        bill_service::update_bill_with_receipt(&state.db, state.files.as_ref(), id, input, receipt)
            .await?,
    ))
}

/// PUT /api/bills/{id}/payment (admin, or the bill's tenant until it has a
/// receipt; multipart with one `payment_file` part)
pub async fn upload_payment_handler(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    path: ValidPath<BillId>,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<Json<BillWithChargesAndReading>, ApiError> {
    let ValidPath(BillId { id }) = path;
    let mut multipart =
        multipart.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let mut payment = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("payment_file") {
            continue; // Other parts are ignored.
        }
        if field.file_name().is_none() {
            return Err(ApiError::BadRequest(
                "payment_file must be a file part with a filename".into(),
            ));
        }
        let bytes = field.bytes().await.map_err(multipart_error)?;
        payment = Some(PaymentUpload {
            bytes: bytes.to_vec(),
        });
    }
    let payment = payment.ok_or_else(|| ApiError::BadRequest("payment_file is required".into()))?;
    Ok(Json(
        bill_service::upload_payment(&state.db, state.files.as_ref(), &claims, id, payment).await?,
    ))
}

/// DELETE /api/bills/{id}/payment (admin)
pub async fn clear_payment_handler(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<BillId>,
) -> Result<Json<BillWithChargesAndReading>, ApiError> {
    let ValidPath(BillId { id }) = path;
    Ok(Json(
        bill_service::clear_payment(&state.db, state.files.as_ref(), id).await?,
    ))
}

/// DELETE /api/bills/{id} (admin)
pub async fn delete_bill(
    State(state): State<AppState>,
    _admin: Admin,
    path: ValidPath<BillId>,
) -> Result<StatusCode, ApiError> {
    let ValidPath(BillId { id }) = path;
    bill_service::delete_bill_with_charges(&state.db, state.files.as_ref(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
