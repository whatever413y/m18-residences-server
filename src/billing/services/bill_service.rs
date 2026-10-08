//! Bills with their additional charges: reads assembled in 3 queries, writes
//! as atomic statement lists (D1 has no interactive transactions), receipt and
//! payment-image uploads to file storage. Each bill keeps the full key its
//! files were stored under (`receipt_key`, `payment_key`).
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
    additional_charge_repo,
    bill_repo::{self, BillFilter},
    reading_read_repo, tenant_read_repo,
};
use crate::billing::services::archive::archive_file;

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

/// One of a bill's two files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BillFile {
    /// The admin's receipt (the bill is paid).
    Receipt,
    /// The tenant's proof of payment.
    Payment,
}

impl BillFile {
    /// The folder new files of this kind are stored in, under the tenant's name.
    fn folder(self) -> &'static str {
        match self {
            Self::Receipt => "receipts",
            Self::Payment => "tenant-payments",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Receipt => "Receipt",
            Self::Payment => "Payment image",
        }
    }

    /// The bill's file name and stored key of this kind.
    fn of(self, bill: &bill::Model) -> (Option<&str>, Option<&str>) {
        let (name, key) = match self {
            Self::Receipt => (&bill.receipt_url, &bill.receipt_key),
            Self::Payment => (&bill.payment_url, &bill.payment_key),
        };
        (
            name.as_deref().filter(|n| !n.is_empty()),
            key.as_deref().filter(|k| !k.is_empty()),
        )
    }

    /// Where a new file of this kind is stored: `<folder>/<tenant name>/<file name>`.
    fn new_key(self, tenant_name: &str, file_name: &str) -> String {
        format!("{}/{tenant_name}/{file_name}", self.folder())
    }
}

#[derive(Debug, Clone)]
pub struct BillInput {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Vec<AdditionalChargeInput>,
    /// The receipt's file name; `None` (or empty) means not paid. A JSON update may only keep the
    /// bill's receipt or clear it; a new one comes with an upload.
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

/// What an update does to the bill's receipt key.
enum ReceiptKey {
    /// The receipt stays (the key is not touched).
    Keep,
    /// The receipt is cleared.
    Clear,
    /// A new receipt was stored at this key.
    New(String),
}

/// A JSON update's `receipt_url` may only keep the bill's receipt or clear it (400 otherwise):
/// receipts are stored by the upload, which also records their key.
fn receipt_change(old: &bill::Model, receipt_url: Option<&str>) -> Result<ReceiptKey, ApiError> {
    match receipt_url.filter(|r| !r.is_empty()) {
        None => Ok(ReceiptKey::Clear),
        Some(name) if BillFile::Receipt.of(old).0 == Some(name) => Ok(ReceiptKey::Keep),
        Some(_) => Err(ApiError::BadRequest(
            "receipt_url can only keep or clear the bill's receipt; upload a new receipt instead"
                .into(),
        )),
    }
}

/// The statements that replace bill `id`'s columns and charges.
fn update_statements(
    db: &Db,
    id: i32,
    input: &BillInput,
    receipt_key: ReceiptKey,
) -> Result<Vec<Statement>, ApiError> {
    let backend = db.backend();
    let mut bill = build_bill_active_model(input)?;
    match receipt_key {
        ReceiptKey::Keep => {}
        ReceiptKey::Clear => bill.receipt_key = Set(None),
        ReceiptKey::New(key) => bill.receipt_key = Set(Some(key)),
    }
    let mut statements = vec![
        bill_repo::update_statement(backend, id, bill),
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

/// The bills matching `filter` (every bill when it's empty) with charges and reading, newest first.
pub async fn get_bills_with_details(
    db: &Db,
    filter: &BillFilter,
) -> Result<Vec<BillWithChargesAndReading>, ApiError> {
    let bills = bill_repo::get_filtered(db.conn(), filter).await?;
    log_ok!("get_bills: fetched {} bills ({filter:?})", bills.len());
    with_details(db, bills).await
}

/// The years bills were created in, newest first.
pub async fn get_bill_years(db: &Db) -> Result<Vec<i32>, ApiError> {
    Ok(bill_repo::get_years(db.conn()).await?)
}

/// The key `bill`'s `kind` file is stored at, or `None` if it has none. Bills
/// whose file came before their key was recorded (migration 0004 backfilled
/// the rest) get it rebuilt from their tenant's current name.
pub async fn file_key(
    db: &Db,
    bill: &bill::Model,
    kind: BillFile,
) -> Result<Option<String>, ApiError> {
    match kind.of(bill) {
        (None, _) => Ok(None),
        (Some(_), Some(key)) => Ok(Some(key.to_string())),
        (Some(name), None) => {
            let tenant = tenant_read_repo::get_by_id(db.conn(), bill.tenant_id)
                .await?
                .ok_or_else(|| {
                    ApiError::Internal(format!(
                        "bill {}'s tenant {} is gone",
                        bill.id, bill.tenant_id
                    ))
                })?;
            Ok(Some(kind.new_key(&tenant.name, name)))
        }
    }
}

/// The key of bill `id`'s `kind` file, for the admin or the bill's tenant
/// (403 for another tenant; 404 if there's no such bill or file).
pub async fn file_key_for(
    db: &Db,
    claims: &Claims,
    id: i32,
    kind: BillFile,
) -> Result<String, ApiError> {
    let bill = bill_repo::get_by_id(db.conn(), id)
        .await?
        .ok_or_else(|| not_found(id))?;
    claims.ensure_admin_or_tenant(bill.tenant_id)?;
    file_key(db, &bill, kind)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{} not found", kind.label())))
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

/// Archives `old`'s receipt (see [`archive_file`]) once no bill points at it
/// (`new_key` is the bill's receipt key now; `None` after a clear or delete).
/// Called only after the database write succeeded, so a failure here leaves
/// the file at its key (logged), never a bill without its receipt.
async fn remove_replaced_receipt(
    db: &Db,
    files: &dyn FileStore,
    old: &bill::Model,
    new_key: Option<&str>,
) {
    let key = match file_key(db, old, BillFile::Receipt).await {
        Ok(Some(key)) => key,
        Ok(None) => return,
        Err(err) => {
            log_error!("The old receipt of bill {} is orphaned: {err}", old.id);
            return;
        }
    };
    if new_key == Some(key.as_str()) {
        return;
    }
    match bill_repo::receipt_in_use(db.conn(), &key).await {
        Ok(false) => {}
        Ok(true) => {
            log_warn!("Kept receipt {key}: another bill still has it");
            return;
        }
        Err(err) => {
            log_error!(
                "Receipt {key} is orphaned: checking whether another bill has it failed ({err})"
            );
            return;
        }
    }
    match archive_file(files, &key).await {
        Ok(()) => log_ok!("Archived receipt {key} of bill {}", old.id),
        Err(err) => log_error!(
            "Receipt {key} of bill {} is orphaned: archiving it failed ({})",
            old.id,
            err.0
        ),
    }
}

/// Replaces a bill's columns and charges in one atomic batch; a receipt the
/// bill no longer has is archived.
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
    let receipt = receipt_change(&old, input.receipt_url.as_deref())?;
    let cleared = matches!(receipt, ReceiptKey::Clear);
    check_references(db, &input, Some(id)).await?;
    db.atomic(update_statements(db, id, &input, receipt)?)
        .await?;
    if cleared {
        remove_replaced_receipt(db, files, &old, None).await;
    }

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
/// the bill's receipt (the form's `receipt_url` is then ignored). The bill and
/// its references are checked before anything is stored; the stored file is
/// removed again if the update still fails; once it succeeds, the receipt it
/// replaced is archived.
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
    check_references(db, &input, Some(id)).await?;
    let tenant = tenant_read_repo::get_by_id(db.conn(), input.tenant_id)
        .await?
        .ok_or_else(|| ApiError::Conflict(format!("Tenant {} does not exist", input.tenant_id)))?;

    let file_name = format!("{}-r{}", Utc::now().timestamp(), input.reading_id);
    let key = BillFile::Receipt.new_key(&tenant.name, &file_name);
    files.put(&key, receipt.bytes, content_type).await?;
    log_ok!("Stored receipt {key} ({content_type})");

    input.receipt_url = Some(file_name);
    let written = match update_statements(db, id, &input, ReceiptKey::New(key.clone())) {
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
    remove_replaced_receipt(db, files, &old, Some(&key)).await;

    let updated = read_back(db, id).await?;
    log_ok!(
        "Updated bill id={} with {} charges and a receipt",
        id,
        updated.additional_charges.len()
    );
    Ok(updated)
}

/// Archives `old`'s payment image (see [`archive_file`]) unless it is the bill's
/// payment image now (`new_key`). Called only after the database write
/// succeeded, so a failure leaves the file at its key (logged).
async fn remove_replaced_payment(
    db: &Db,
    files: &dyn FileStore,
    old: &bill::Model,
    new_key: Option<&str>,
) {
    let key = match file_key(db, old, BillFile::Payment).await {
        Ok(Some(key)) => key,
        Ok(None) => return,
        Err(err) => {
            log_error!(
                "The old payment image of bill {} is orphaned: {err}",
                old.id
            );
            return;
        }
    };
    // The same key only when re-uploaded within the second: `put` already replaced it.
    if new_key == Some(key.as_str()) {
        return;
    }
    match archive_file(files, &key).await {
        Ok(()) => log_ok!("Archived payment image {key} of bill {}", old.id),
        Err(err) => log_error!(
            "Payment image {key} of bill {} is orphaned: archiving it failed ({})",
            old.id,
            err.0
        ),
    }
}

/// Removes a just-stored file whose bill update did not happen.
async fn remove_unattached(files: &dyn FileStore, key: &str, id: i32) {
    match files.delete(key).await {
        Ok(()) => log_warn!("Removed {key}: the bill {id} update did not happen"),
        Err(err) => log_error!(
            "{key} is orphaned: the bill {id} update did not happen and its removal failed ({})",
            err.0
        ),
    }
}

/// Stores `payment` as bill `id`'s proof of payment under
/// `tenant-payments/<tenant name>/<unix seconds>-r<reading id>`, replacing
/// (and then archiving) the previous one. Admins may always do this; the
/// bill's tenant only while the bill has no receipt (409 after), checked in
/// the same statement that attaches the file.
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
    let tenant_only = !claims.is_admin();
    let already_paid = || ApiError::Conflict("This bill is already paid".into());
    if tenant_only && BillFile::Receipt.of(&old).0.is_some() {
        return Err(already_paid());
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
    let key = BillFile::Payment.new_key(&tenant.name, &file_name);
    files.put(&key, payment.bytes, content_type).await?;
    log_ok!("Stored payment image {key} ({content_type})");

    let changed =
        bill_repo::set_payment(db.conn(), id, Some((file_name, key.clone())), tenant_only).await;
    match changed {
        Ok(0) => {
            remove_unattached(files, &key, id).await;
            // Deleted, or given a receipt, since it was read.
            return Err(match bill_repo::get_by_id(db.conn(), id).await? {
                None => not_found(id),
                Some(_) => already_paid(),
            });
        }
        Ok(_) => {}
        Err(err) => {
            remove_unattached(files, &key, id).await;
            return Err(err.into());
        }
    }
    remove_replaced_payment(db, files, &old, Some(&key)).await;
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
    if bill_repo::set_payment(db.conn(), id, None, false).await? == 0 {
        return Err(not_found(id));
    }
    remove_replaced_payment(db, files, &old, None).await;
    log_ok!("Cleared the payment image of bill id={id}");
    read_back(db, id).await
}

/// Deletes a bill and its charges in one atomic batch, then archives its receipt and payment files.
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
    remove_replaced_payment(db, files, &old, None).await;
    Ok(())
}
