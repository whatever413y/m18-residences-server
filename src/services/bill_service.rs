use crate::{
    entities::{additional_charge, bill, electricity_reading},
    repository::{additional_charge_repo, bill_repo},
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, DbErr, Set, TransactionError, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Serialize)]
pub struct BillWithCharges {
    pub bill: bill::Model,
    pub additional_charges: Vec<additional_charge::Model>,
}

#[derive(Serialize)]
pub struct BillWithChargesAndReading {
    pub bill: bill::Model,
    pub additional_charges: Vec<additional_charge::Model>,
    pub reading: Option<electricity_reading::Model>,
}

#[derive(Clone, Deserialize)]
pub struct AdditionalChargeInput {
    pub amount: i32,
    pub description: String,
}

#[derive(Clone)]
pub struct BillInput {
    pub tenant_id: i32,
    pub reading_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub additional_charges: Vec<AdditionalChargeInput>,
    pub receipt_url: Option<String>,
}

// ---------- helpers ----------
fn calculate_total(room: i32, electric: i32, charges: &[AdditionalChargeInput]) -> i32 {
    room + electric + charges.iter().map(|c| c.amount).sum::<i32>()
}

fn determine_paid_status(receipt_url: &Option<String>) -> bool {
    receipt_url.is_some()
}

fn build_bill_active_model(input: &BillInput) -> bill::ActiveModel {
    bill::ActiveModel {
        tenant_id: Set(input.tenant_id),
        reading_id: Set(input.reading_id),
        room_charges: Set(input.room_charges),
        electric_charges: Set(input.electric_charges),
        total_amount: Set(calculate_total(input.room_charges, input.electric_charges, &input.additional_charges)),
        paid: Set(determine_paid_status(&input.receipt_url)),
        receipt_url: Set(input.receipt_url.clone()),
        ..Default::default()
    }
}

fn build_charge_models(bill_id: i32, charges: &[AdditionalChargeInput]) -> Vec<additional_charge::ActiveModel> {
    let now = Utc::now().naive_utc();
    charges.iter().map(|c| additional_charge::ActiveModel {
        bill_id: Set(bill_id),
        amount: Set(c.amount),
        description: Set(c.description.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }).collect()
}

// Insert multiple charges via repo
async fn insert_charges(txn: &DatabaseTransaction, bill_id: i32, charges: &[AdditionalChargeInput]) -> Result<(), DbErr> {
    for charge in build_charge_models(bill_id, charges) {
        // Use the repo's create function
        additional_charge_repo::create(txn, charge).await?;
    }
    Ok(())
}

// Delete all charges for a bill via repo
async fn delete_charges(txn: &DatabaseTransaction, bill_id: i32) -> Result<(), DbErr> {
    additional_charge_repo::delete_many_by_bill_id(txn, bill_id).await
}

fn map_txn_err<T>(res: Result<T, TransactionError<DbErr>>) -> Result<T, DbErr> {
    res.map_err(|e| match e {
        TransactionError::Connection(err) => err,
        TransactionError::Transaction(err) => err,
    })
}

// ---------- public methods ----------

pub async fn get_bills_for_tenant(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<BillWithChargesAndReading>, DbErr> {
    let raw_data = bill_repo::get_bills_with_readings_and_charges_by_tenant(db, tenant_id).await?;

    let result = raw_data.into_iter().map(|(bill, reading, charges)| BillWithChargesAndReading {
        bill,
        additional_charges: charges,
        reading,
    }).collect::<Vec<_>>();

    println!("✅ get_bills_for_tenant: fetched {} bills for tenant {}", result.len(), tenant_id);

    Ok(result)
}

pub async fn create_bill(db: &DatabaseConnection, input: BillInput) -> Result<bill::Model, DbErr> {
    map_txn_err(
        db.transaction::<_, bill::Model, DbErr>(|txn| {
            let input = input.clone();
            Box::pin(async move {
                let bill_model = build_bill_active_model(&input).insert(txn).await?;
                insert_charges(txn, bill_model.id, &input.additional_charges).await?;
                Ok(bill_model)
            })
        }).await
    )
}

pub async fn update_bill(db: &DatabaseConnection, id: i32, input: BillInput) -> Result<bill::Model, DbErr> {
    map_txn_err(
        db.transaction::<_, bill::Model, DbErr>(|txn| {
            let input = input.clone();
            Box::pin(async move {
                let mut bill_am = build_bill_active_model(&input);
                bill_am.id = Set(id);
                let updated_bill = bill_am.update(txn).await?;

                delete_charges(txn, updated_bill.id).await?;
                insert_charges(txn, updated_bill.id, &input.additional_charges).await?;

                Ok(updated_bill)
            })
        }).await
    )
}
