use crate::{
    entities::{additional_charge, bill, electricity_reading},
    repository::{additional_charge_repo, bill_repo, electricity_reading_repo},
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, DbErr, Set, TransactionError,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};

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
        total_amount: Set(calculate_total(
            input.room_charges,
            input.electric_charges,
            &input.additional_charges,
        )),
        paid: Set(determine_paid_status(&input.receipt_url)),
        receipt_url: Set(input.receipt_url.clone()),
        ..Default::default()
    }
}

fn build_charge_models(
    bill_id: i32,
    charges: &[AdditionalChargeInput],
) -> Vec<additional_charge::ActiveModel> {
    charges
        .iter()
        .map(|c| additional_charge::ActiveModel {
            bill_id: Set(bill_id),
            amount: Set(c.amount),
            description: Set(c.description.clone()),
            ..Default::default()
        })
        .collect()
}

async fn insert_charges(
    txn: &DatabaseTransaction,
    bill_id: i32,
    charges: &[AdditionalChargeInput],
) -> Result<(), DbErr> {
    for charge in build_charge_models(bill_id, charges) {
        additional_charge_repo::create(txn, charge).await?;
    }
    Ok(())
}

fn map_txn_err<T>(res: Result<T, TransactionError<DbErr>>) -> Result<T, DbErr> {
    res.map_err(|e| match e {
        TransactionError::Connection(err) => err,
        TransactionError::Transaction(err) => err,
    })
}

// ---------- public methods ----------

/// GET all bills with charges and reading
pub async fn get_all_bills_with_details(
    db: &DatabaseConnection,
) -> Result<Vec<BillWithChargesAndReading>, DbErr> {
    let bills = bill_repo::get_all(db).await?;
    let mut result = Vec::with_capacity(bills.len());

    for bill_model in bills {
        let charges = additional_charge_repo::get_all_by_bill_id(db, bill_model.id).await?;
        let reading = electricity_reading_repo::get_by_id(db, bill_model.reading_id).await?;

        result.push(BillWithChargesAndReading {
            bill: bill_model,
            additional_charges: charges,
            reading,
        });
    }

    Ok(result)
}

/// GET the most recent bill for a tenant with charges and reading
pub async fn get_tenant_bill_with_details(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Option<BillWithChargesAndReading>, DbErr> {
    if let Some(bill_model) = bill_repo::get_latest_by_tenant_id(db, tenant_id).await? {
        let charges = additional_charge_repo::get_all_by_bill_id(db, bill_model.id).await?;
        let reading = electricity_reading_repo::get_by_id(db, bill_model.reading_id).await?;

        Ok(Some(BillWithChargesAndReading {
            bill: bill_model,
            additional_charges: charges,
            reading,
        }))
    } else {
        Ok(None)
    }
}

/// GET all the bills for a tenant with charges and readings
pub async fn get_all_bills_for_tenant(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<BillWithChargesAndReading>, DbErr> {
    let bills = bill_repo::get_all_by_tenant_id(db, tenant_id).await?;
    let mut result = Vec::with_capacity(bills.len());

    for bill_model in bills {
        let charges = additional_charge_repo::get_all_by_bill_id(db, bill_model.id).await?;
        let reading = electricity_reading_repo::get_by_id(db, bill_model.reading_id).await?;

        result.push(BillWithChargesAndReading {
            bill: bill_model,
            additional_charges: charges,
            reading,
        });
    }

    Ok(result)
}


// CREATE a new bill
pub async fn create_bill(
    db: &DatabaseConnection,
    input: BillInput,
) -> Result<BillWithChargesAndReading, DbErr> {
    map_txn_err(
        db.transaction::<_, BillWithChargesAndReading, DbErr>(|txn| {
            let input = input.clone();
            Box::pin(async move {
                let bill_model = build_bill_active_model(&input).insert(txn).await?;
                insert_charges(txn, bill_model.id, &input.additional_charges).await?;

                let charges = additional_charge_repo::get_all_by_bill_id(txn, bill_model.id).await?;
                let reading =
                    electricity_reading_repo::get_by_id(txn, bill_model.reading_id).await?;

                println!(
                    "✅ Created bill id={} with {} charges",
                    bill_model.id,
                    charges.len()
                );

                Ok(BillWithChargesAndReading {
                    bill: bill_model,
                    additional_charges: charges,
                    reading,
                })
            })
        })
        .await,
    )
}

// UPDATE a bill
pub async fn update_bill(
    db: &DatabaseConnection,
    id: i32,
    input: BillInput,
) -> Result<BillWithChargesAndReading, DbErr> {
    map_txn_err(
        db.transaction::<_, BillWithChargesAndReading, DbErr>(|txn| {
            let input = input.clone();
            Box::pin(async move {
                let mut bill_am = build_bill_active_model(&input);
                bill_am.id = Set(id);
                let updated_bill = bill_am.update(txn).await?;

                additional_charge_repo::delete_many_by_bill_id(txn, updated_bill.id).await?;
                insert_charges(txn, updated_bill.id, &input.additional_charges).await?;

                let charges =
                    additional_charge_repo::get_all_by_bill_id(txn, updated_bill.id).await?;
                let reading =
                    electricity_reading_repo::get_by_id(txn, updated_bill.reading_id).await?;

                println!(
                    "✅ Updated bill id={} with {} charges",
                    updated_bill.id,
                    charges.len()
                );

                Ok(BillWithChargesAndReading {
                    bill: updated_bill,
                    additional_charges: charges,
                    reading,
                })
            })
        })
        .await,
    )
}

// Delete bill and additional charges
pub async fn delete_bill_with_charges(
    db: &DatabaseConnection,
    bill_id: i32,
) -> Result<Option<bill::Model>, DbErr> {
    map_txn_err(
        db.transaction::<_, Option<bill::Model>, DbErr>(|txn| {
            Box::pin(async move {
                let deleted_charges =
                    additional_charge_repo::delete_many_by_bill_id(txn, bill_id).await?;
                let deleted_bill = bill_repo::delete(txn, bill_id).await?;

                if let Some(bill) = &deleted_bill {
                    println!(
                        "✅ Deleted bill id={} with {} charges",
                        bill.id, deleted_charges
                    );
                }

                Ok(deleted_bill)
            })
        })
        .await,
    )
}
