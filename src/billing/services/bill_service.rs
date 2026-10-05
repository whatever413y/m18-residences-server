//! Bills with their additional charges: reads assembled in 3 queries, writes
//! as atomic statement lists (D1 has no interactive transactions), receipt and
//! payment-image uploads to file storage.
use std::collections::HashMap;

use chrono::Utc;
use m18_residences_db::{
    Db,
    entities::{additional_charge, bill, electricity_reading},
};
use m18_residences_shared_rs::{
    auth::Claims,
    error::ApiError,
    files::{FileStore, sniff},
    log_error, log_ok, log_warn,
};
use sea_orm::{Set, Statement};
use serde::{Deserialize, Serialize};

use crate::billing::repository::{
    additional_charge_repo, bill_repo, reading_read_repo, tenant_read_repo,
};

/// The API shape of a bill: `{bill, additional_charges, reading}`.
#[derive(Debug, Serialize)]
pub struct BillWithChargesAndReading {
    pub bill: bill::Model,
    pub additional_charges: Vec<additional_charge::Model>,
    pub reading: Option<electricity_reading::Model>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdditionalChargeInput {
    pub amount: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct BillInput {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Vec<AdditionalChargeInput>,
    /// The receipt's file name; `None` (or empty) means not paid.
    pub receipt_url: Option<String>,
}

/// A receipt file to store with a bill update.
pub struct ReceiptUpload {
    pub bytes: Vec<u8>,
}

/// A tenant's proof of payment to store with a bill.
pub struct PaymentUpload {
    pub bytes: Vec<u8>,
}

/// The receipt and payment-image types accepted, as sniffed from the file's bytes.
const RECEIPT_TYPES: [&str; 6] = [
    "image/webp",
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/avif",
    "application/pdf",
];

// ---------- helpers ----------

fn calculate_total(
    room: i32,
    electric: i32,
    charges: &[AdditionalChargeInput],
) -> Result<i32, ApiError> {
    room.checked_add(electric)
        .and_then(|base| {
            charges
                .iter()
                .try_fold(base, |total, c| total.checked_add(c.amount))
        })
        .ok_or_else(|| ApiError::BadRequest("The total amount is too large".into()))
}

/// An empty receipt means no receipt.
fn normalize_receipt(receipt_url: Option<String>) -> Option<String> {
    receipt_url.filter(|r| !r.is_empty())
}

fn build_bill_active_model(input: &BillInput) -> Result<bill::ActiveModel, ApiError> {
    let receipt_url = normalize_receipt(input.receipt_url.clone());
    Ok(bill::ActiveModel {
        tenant_id: Set(input.tenant_id),
        reading_id: Set(input.reading_id),
        room_charges: Set(input.room_charges),
        electric_charges: Set(input.electric_charges),
        total_amount: Set(calculate_total(
            input.room_charges,
            input.electric_charges,
            &input.additional_charges,
        )?),
        paid: Set(receipt_url.is_some()),
        receipt_url: Set(receipt_url),
        ..Default::default()
    })
}

/// Input checks that need no database.
fn validate(input: &BillInput) -> Result<(), ApiError> {
    calculate_total(
        input.room_charges,
        input.electric_charges,
        &input.additional_charges,
    )
    .map(|_| ())
}

/// Where a bill's payment image `file_name` is stored.
fn payment_key(tenant_name: &str, file_name: &str) -> String {
    format!("tenant-payments/{tenant_name}/{file_name}")
}

fn not_found(id: i32) -> ApiError {
    ApiError::NotFound(format!("Bill {id} not found"))
}

/// Assembles bills with their charges and readings: one query for all the
/// charges and one for all the readings (per 100 bills), never one per bill.
async fn with_details(
    db: &Db,
    bills: Vec<bill::Model>,
) -> Result<Vec<BillWithChargesAndReading>, ApiError> {
    if bills.is_empty() {
        return Ok(Vec::new());
    }
    let bill_ids: Vec<i32> = bills.iter().map(|b| b.id).collect();
    let reading_ids: Vec<i32> = bills.iter().map(|b| b.reading_id).collect();

    let mut charges: HashMap<i32, Vec<additional_charge::Model>> = HashMap::new();
    for charge in additional_charge_repo::get_all_by_bill_ids(db.conn(), &bill_ids).await? {
        charges.entry(charge.bill_id).or_default().push(charge);
    }
    let mut readings: HashMap<i32, electricity_reading::Model> =
        reading_read_repo::get_by_ids(db.conn(), &reading_ids)
            .await?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();

    Ok(bills
        .into_iter()
        .map(|bill| BillWithChargesAndReading {
            additional_charges: charges.remove(&bill.id).unwrap_or_default(),
            reading: readings.remove(&bill.reading_id),
            bill,
        })
        .collect())
}

async fn one_with_details(
    db: &Db,
    bill: bill::Model,
) -> Result<BillWithChargesAndReading, ApiError> {
    let id = bill.id;
    with_details(db, vec![bill])
        .await?
        .pop()
        .ok_or_else(|| ApiError::Internal(format!("bill {id} vanished while being read")))
}

/// Specific conflicts for a bill's references, checked before writing (the
/// constraints still guard against races). `bill_id` is the bill being updated.
async fn check_references(
    db: &Db,
    input: &BillInput,
    bill_id: Option<i32>,
) -> Result<(), ApiError> {
    if tenant_read_repo::get_by_id(db.conn(), input.tenant_id)
        .await?
        .is_none()
    {
        return Err(ApiError::Conflict(format!(
            "Tenant {} does not exist",
            input.tenant_id
        )));
    }
    check_reading(db, input.reading_id, bill_id).await
}

async fn check_reading(db: &Db, reading_id: i32, bill_id: Option<i32>) -> Result<(), ApiError> {
    if reading_read_repo::get_by_id(db.conn(), reading_id)
        .await?
        .is_none()
    {
        return Err(ApiError::Conflict(format!(
            "Reading {reading_id} does not exist"
        )));
    }
    if let Some(other) = bill_repo::get_by_reading_id(db.conn(), reading_id).await?
        && Some(other.id) != bill_id
    {
        return Err(ApiError::Conflict(format!(
            "Reading {reading_id} already has a bill"
        )));
    }
    Ok(())
}

/// The statements that replace bill `id`'s columns and charges.
fn update_statements(db: &Db, id: i32, input: &BillInput) -> Result<Vec<Statement>, ApiError> {
    let backend = db.backend();
    let mut statements = vec![
        bill_repo::update_statement(backend, id, build_bill_active_model(input)?),
        additional_charge_repo::delete_by_bill_id_statement(backend, id),
    ];
    statements.extend(
        input.additional_charges.iter().map(|c| {
            additional_charge_repo::insert_statement(backend, id, c.amount, &c.description)
        }),
    );
    Ok(statements)
}

async fn read_back(db: &Db, id: i32) -> Result<BillWithChargesAndReading, ApiError> {
    let bill = bill_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    one_with_details(db, bill).await
}

// ---------- public methods ----------

/// All bills with charges and reading, newest first.
pub async fn get_all_bills_with_details(
    db: &Db,
) -> Result<Vec<BillWithChargesAndReading>, ApiError> {
    let bills = bill_repo::get_all(db.conn()).await?;
    with_details(db, bills).await
}

/// A tenant's newest bill with charges and reading.
pub async fn get_tenant_bill_with_details(
    db: &Db,
    tenant_id: i32,
) -> Result<Option<BillWithChargesAndReading>, ApiError> {
    match bill_repo::get_latest_by_tenant_id(db.conn(), tenant_id).await? {
        Some(bill) => Ok(Some(one_with_details(db, bill).await?)),
        None => Ok(None),
    }
}

/// A tenant's bills with charges and readings, newest first.
pub async fn get_all_bills_for_tenant(
    db: &Db,
    tenant_id: i32,
) -> Result<Vec<BillWithChargesAndReading>, ApiError> {
    let bills = bill_repo::get_all_by_tenant_id(db.conn(), tenant_id).await?;
    with_details(db, bills).await
}

/// Creates a bill and its charges in one atomic batch. A new bill never has a receipt.
pub async fn create_bill(
    db: &Db,
    mut input: BillInput,
) -> Result<BillWithChargesAndReading, ApiError> {
    input.receipt_url = None;
    validate(&input)?;
    check_references(db, &input, None).await?;

    let backend = db.backend();
    let mut statements = vec![bill_repo::insert_statement(
        backend,
        build_bill_active_model(&input)?,
    )];
    statements.extend(input.additional_charges.iter().map(|c| {
        additional_charge_repo::insert_for_reading_statement(
            backend,
            input.reading_id,
            c.amount,
            &c.description,
        )
    }));
    db.atomic(statements).await?;

    let bill = bill_repo::get_by_reading_id(db.conn(), input.reading_id)
        .await?
        .ok_or_else(|| {
            ApiError::Internal(format!(
                "the bill for reading {} vanished after its insert",
                input.reading_id
            ))
        })?;
    let created = one_with_details(db, bill).await?;
    log_ok!(
        "Created bill id={} with {} charges",
        created.bill.id,
        created.additional_charges.len()
    );
    Ok(created)
}

/// Removes `old`'s receipt from file storage once the bill no longer points at
/// it (`new_receipt` is the bill's receipt now; `None` after a delete). Called
/// only after the database write succeeded, so a failure here leaves an
/// orphaned file (logged), never a bill without its receipt.
async fn remove_replaced_receipt(
    db: &Db,
    files: &dyn FileStore,
    old: &bill::Model,
    new_receipt: Option<&str>,
) {
    let Some(old_receipt) = old.receipt_url.as_deref().filter(|r| !r.is_empty()) else {
        return;
    };
    if new_receipt == Some(old_receipt) {
        return;
    }
    let tenant = match tenant_read_repo::get_by_id(db.conn(), old.tenant_id).await {
        Ok(Some(tenant)) => tenant,
        Ok(None) => {
            log_error!(
                "Receipt {old_receipt} of bill {} is orphaned: its tenant {} is gone",
                old.id,
                old.tenant_id
            );
            return;
        }
        Err(err) => {
            log_error!(
                "Receipt {old_receipt} of bill {} is orphaned: reading its tenant failed ({err})",
                old.id
            );
            return;
        }
    };
    let key = format!("receipts/{}/{old_receipt}", tenant.name);
    match bill_repo::receipt_in_use(db.conn(), old.tenant_id, old_receipt).await {
        Ok(false) => {}
        Ok(true) => {
            log_warn!("Kept receipt {key}: another bill of the tenant still has it");
            return;
        }
        Err(err) => {
            log_error!(
                "Receipt {key} is orphaned: checking whether another bill has it failed ({err})"
            );
            return;
        }
    }
    match files.delete(&key).await {
        Ok(()) => log_ok!("Removed receipt {key} of bill {}", old.id),
        Err(err) => log_error!(
            "Receipt {key} of bill {} is orphaned: removing it failed ({})",
            old.id,
            err.0
        ),
    }
}

/// Replaces a bill's columns and charges in one atomic batch; a receipt the
/// bill no longer has is removed from file storage.
pub async fn update_bill(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
    input: BillInput,
) -> Result<BillWithChargesAndReading, ApiError> {
    validate(&input)?;
    let Some(old) = bill_repo::get_by_id(db.conn(), id).await? else {
        return Err(not_found(id));
    };
    check_references(db, &input, Some(id)).await?;
    db.atomic(update_statements(db, id, &input)?).await?;
    let new_receipt = normalize_receipt(input.receipt_url.clone());
    remove_replaced_receipt(db, files, &old, new_receipt.as_deref()).await;

    let updated = read_back(db, id).await?;
    log_ok!(
        "Updated bill id={} with {} charges",
        id,
        updated.additional_charges.len()
    );
    Ok(updated)
}

/// Like [`update_bill`], first storing `receipt` (if any) under
/// `receipts/<tenant name>/<unix seconds>-r<reading id>`, which then becomes
/// the bill's receipt. The stored file is removed again if the update fails;
/// once it succeeds, the receipt it replaced is removed.
pub async fn update_bill_with_receipt(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
    mut input: BillInput,
    receipt: Option<ReceiptUpload>,
) -> Result<BillWithChargesAndReading, ApiError> {
    let Some(receipt) = receipt else {
        return update_bill(db, files, id, input).await;
    };
    validate(&input)?;
    let content_type = sniff(&receipt.bytes)
        .filter(|t| RECEIPT_TYPES.contains(t))
        .ok_or_else(|| ApiError::BadRequest("Unsupported receipt type".into()))?;
    let Some(old) = bill_repo::get_by_id(db.conn(), id).await? else {
        return Err(not_found(id));
    };
    let tenant = tenant_read_repo::get_by_id(db.conn(), input.tenant_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("Tenant {} not found", input.tenant_id)))?;

    let file_name = format!("{}-r{}", Utc::now().timestamp(), input.reading_id);
    let key = format!("receipts/{}/{file_name}", tenant.name);
    files.put(&key, receipt.bytes, content_type).await?;
    log_ok!("Stored receipt {key} ({content_type})");

    input.receipt_url = Some(file_name);
    let written = match update_statements(db, id, &input) {
        Ok(statements) => db.atomic(statements).await.map_err(ApiError::from),
        Err(err) => Err(err),
    };
    if let Err(err) = written {
        match files.delete(&key).await {
            Ok(()) => log_warn!("Removed receipt {key}: the bill {id} update failed"),
            Err(delete_err) => log_error!(
                "Receipt {key} is orphaned: the bill {id} update failed and so did its removal ({})",
                delete_err.0
            ),
        }
        return Err(err);
    }
    remove_replaced_receipt(db, files, &old, input.receipt_url.as_deref()).await;

    let updated = read_back(db, id).await?;
    log_ok!(
        "Updated bill id={} with {} charges and a receipt",
        id,
        updated.additional_charges.len()
    );
    Ok(updated)
}

/// Removes `old`'s payment image from file storage once the bill no longer
/// points at it. Called only after the database write succeeded, so a failure
/// leaves an orphaned file (logged).
async fn remove_replaced_payment(db: &Db, files: &dyn FileStore, old: &bill::Model) {
    let Some(old_payment) = old.payment_url.as_deref().filter(|p| !p.is_empty()) else {
        return;
    };
    let tenant = match tenant_read_repo::get_by_id(db.conn(), old.tenant_id).await {
        Ok(Some(tenant)) => tenant,
        Ok(None) => {
            log_error!(
                "Payment image {old_payment} of bill {} is orphaned: its tenant {} is gone",
                old.id,
                old.tenant_id
            );
            return;
        }
        Err(err) => {
            log_error!(
                "Payment image {old_payment} of bill {} is orphaned: reading its tenant failed ({err})",
                old.id
            );
            return;
        }
    };
    let key = payment_key(&tenant.name, old_payment);
    match files.delete(&key).await {
        Ok(()) => log_ok!("Removed payment image {key} of bill {}", old.id),
        Err(err) => log_error!(
            "Payment image {key} of bill {} is orphaned: removing it failed ({})",
            old.id,
            err.0
        ),
    }
}

/// Stores `payment` as bill `id`'s proof of payment under
/// `tenant-payments/<tenant name>/<unix seconds>-r<reading id>`, replacing
/// (and then removing) the previous one. Admins may always do this; the
/// bill's tenant only until the bill has a receipt (409 after).
pub async fn upload_payment(
    db: &Db,
    files: &dyn FileStore,
    claims: &Claims,
    id: i32,
    payment: PaymentUpload,
) -> Result<BillWithChargesAndReading, ApiError> {
    let Some(old) = bill_repo::get_by_id(db.conn(), id).await? else {
        return Err(not_found(id));
    };
    claims.ensure_admin_or_tenant(old.tenant_id)?;
    if !claims.is_admin() && old.receipt_url.as_deref().is_some_and(|r| !r.is_empty()) {
        return Err(ApiError::Conflict("This bill is already paid".into()));
    }
    let content_type = sniff(&payment.bytes)
        .filter(|t| RECEIPT_TYPES.contains(t))
        .ok_or_else(|| ApiError::BadRequest("Unsupported payment image type".into()))?;
    let tenant = tenant_read_repo::get_by_id(db.conn(), old.tenant_id)
        .await?
        .ok_or_else(|| {
            ApiError::Internal(format!("bill {id}'s tenant {} is gone", old.tenant_id))
        })?;

    let file_name = format!("{}-r{}", Utc::now().timestamp(), old.reading_id);
    let key = payment_key(&tenant.name, &file_name);
    files.put(&key, payment.bytes, content_type).await?;
    log_ok!("Stored payment image {key} ({content_type})");

    let statement = bill_repo::set_payment_statement(db.backend(), id, Some(file_name.clone()));
    if let Err(err) = db.atomic(vec![statement]).await {
        match files.delete(&key).await {
            Ok(()) => log_warn!("Removed payment image {key}: the bill {id} update failed"),
            Err(delete_err) => log_error!(
                "Payment image {key} is orphaned: the bill {id} update failed and so did its removal ({})",
                delete_err.0
            ),
        }
        return Err(err.into());
    }
    // The same name only when re-uploaded within the second: `put` already replaced it.
    if old.payment_url.as_deref() != Some(file_name.as_str()) {
        remove_replaced_payment(db, files, &old).await;
    }
    log_ok!("Bill id={id} has a new payment image");
    read_back(db, id).await
}

/// Clears bill `id`'s payment image, then removes its file.
pub async fn clear_payment(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
) -> Result<BillWithChargesAndReading, ApiError> {
    let Some(old) = bill_repo::get_by_id(db.conn(), id).await? else {
        return Err(not_found(id));
    };
    db.atomic(vec![bill_repo::set_payment_statement(
        db.backend(),
        id,
        None,
    )])
    .await?;
    remove_replaced_payment(db, files, &old).await;
    log_ok!("Cleared the payment image of bill id={id}");
    read_back(db, id).await
}

/// Deletes a bill and its charges in one atomic batch, then its receipt and payment files.
pub async fn delete_bill_with_charges(
    db: &Db,
    files: &dyn FileStore,
    id: i32,
) -> Result<(), ApiError> {
    let Some(old) = bill_repo::get_by_id(db.conn(), id).await? else {
        return Err(not_found(id));
    };
    let backend = db.backend();
    db.atomic(vec![
        additional_charge_repo::delete_by_bill_id_statement(backend, id),
        bill_repo::delete_statement(backend, id),
    ])
    .await?;
    log_ok!("Deleted bill id={id} with its charges");
    remove_replaced_receipt(db, files, &old, None).await;
    remove_replaced_payment(db, files, &old).await;
    Ok(())
}
