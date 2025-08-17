use crate::{entities::{additional_charge, bill, electricity_reading}, repository::bill_repo};
use sea_orm::{DatabaseConnection, DbErr, Set};
use serde::{Deserialize, Serialize};

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

/// Calculate total bill amount including additional charges
pub fn calculate_total(
    room_charges: i32,
    electric_charges: i32,
    additional_charges: &[AdditionalChargeInput],
) -> i32 {
    room_charges + electric_charges + additional_charges.iter().map(|c| c.amount).sum::<i32>()
}

/// Paid status is true if receipt URL exists
pub fn determine_paid_status(receipt_url: &Option<String>) -> bool {
    receipt_url.is_some()
}

/// Build ActiveModel for bill
pub fn build_bill_active_model(input: &BillInput) -> bill::ActiveModel {
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

/// Build ActiveModels for additional charges
pub fn build_additional_charge_active_models(
    bill_id: i32,
    charges: &[AdditionalChargeInput],
) -> Vec<additional_charge::ActiveModel> {
    charges.iter().map(|c| additional_charge::ActiveModel {
        bill_id: Set(bill_id),
        amount: Set(c.amount),
        description: Set(c.description.clone()),
        ..Default::default()
    }).collect()
}

pub async fn get_bills_for_tenant(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<BillWithChargesAndReading>, DbErr> {
    let raw_data = bill_repo::get_bills_with_readings_and_charges_by_tenant(db, tenant_id).await?;

    let mut result = Vec::with_capacity(raw_data.len());

    for (bill_model, reading_opt, charges) in raw_data {
        if charges.is_empty() {
            println!("⚠️ bill id={} has no additional charges, reading exists={}", bill_model.id, reading_opt.is_some());
        } else {
            println!("✅ bill id={} has {} additional charges, reading exists={}", bill_model.id, charges.len(), reading_opt.is_some());
        }

        result.push(BillWithChargesAndReading {
            bill: bill_model,
            additional_charges: charges,
            reading: reading_opt,
        });
    }

    println!("✅ get_bills_for_tenant: fetched {} bills for tenant {}", result.len(), tenant_id);

    Ok(result)
}
