use crate::entities::{bill, additional_charge, electricity_reading};
use crate::services::bill_service::{build_additional_charge_active_models, build_bill_active_model, BillInput, BillWithCharges, BillWithChargesAndReading};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait};

/// GET all bills with their additional charges
pub async fn get_all(db: &DatabaseConnection)
    -> Result<Vec<BillWithCharges>, DbErr>
{
    let list = bill::Entity::find()
        .order_by_desc(bill::Column::CreatedAt)
        .find_with_related(additional_charge::Entity)
        .all(db)
        .await?;

    let wrapped: Vec<BillWithCharges> = list.into_iter()
        .map(|(b, charges)| BillWithCharges {
            bill: b,
            additional_charges: charges,
        })
        .collect();

    println!("✅ get_all: fetched {} bills with additional charges", wrapped.len());

    Ok(wrapped)
}

/// GET bill by ID with additional charges
pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<BillWithCharges>, DbErr> {
    let mut result = bill::Entity::find()
        .filter(bill::Column::Id.eq(id))
        .find_with_related(additional_charge::Entity)
        .all(db)
        .await?;

    let res = result.pop();

    match &res {
        Some((b, _)) => println!("✅ get_by_id: found bill id={}", b.id),
        None => println!("⚠️ get_by_id: bill id={} not found", id),
    }

    Ok(res.map(|(bill_model, charges)| BillWithCharges {
        bill: bill_model,
        additional_charges: charges,
    }))
}

// GET all bills for a specific tenant with additional charges and reading info
pub async fn get_bills_with_readings_and_charges_by_tenant(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<(bill::Model, Option<electricity_reading::Model>, Vec<additional_charge::Model>)>, DbErr> {
    let rows = bill::Entity::find()
        .filter(bill::Column::TenantId.eq(tenant_id))
        .order_by_desc(bill::Column::CreatedAt)
        .find_also_related(electricity_reading::Entity)
        .all(db)
        .await?;

    let mut result = Vec::with_capacity(rows.len());

    for (bill_model, reading_opt) in rows {
        let charges = additional_charge::Entity::find()
            .filter(additional_charge::Column::BillId.eq(bill_model.id))
            .all(db)
            .await?;

        result.push((bill_model, reading_opt, charges));
    }

    Ok(result)
}


// /// CREATE a bill with charges in a transaction
// pub async fn create(
//     db: &DatabaseConnection,
//     input: BillInput
// ) -> Result<bill::Model, DbErr> {
//     db.transaction::<_, bill::Model, DbErr>(|txn| async move {
//         let bill_am = build_bill_active_model(&input);
//         let bill_model = bill_am.insert(txn).await?;

//         let charges = build_additional_charge_active_models(bill_model.id, &input.additional_charges);
//         for ac in charges {
//             ac.insert(txn).await?;
//         }

//         println!("✅ create: created bill id={}", bill_model.id);
//         Ok(bill_model)
//     }).await
// }

// /// UPDATE a bill and its charges in a transaction
// pub async fn update(
//     db: &DatabaseConnection,
//     id: i32,
//     input: BillInput
// ) -> Result<bill::Model, DbErr> {
//     db.transaction::<_, bill::Model, DbErr>(|txn| async move {
//         let mut bill_am = build_bill_active_model(&input);
//         bill_am.id = Set(id);
//         bill_am.updated_at = Set(chrono::Utc::now().naive_utc());

//         let updated_bill = bill_am.update(txn).await?;

//         // Remove old additional charges
//         additional_charge::Entity::delete_many()
//             .filter(additional_charge::Column::BillId.eq(id))
//             .exec(txn)
//             .await?;

//         // Insert new charges
//         let charges = build_additional_charge_active_models(id, &input.additional_charges);
//         for ac in charges {
//             ac.insert(txn).await?;
//         }

//         println!("✅ update: updated bill id={}", updated_bill.id);
//         Ok(updated_bill)
//     }).await
// }

/// DELETE a bill
pub async fn delete(
    db: &DatabaseConnection,
    id: i32
) -> Result<Option<bill::Model>, DbErr> {
    if let Some(model) = bill::Entity::find_by_id(id).one(db).await? {
        let am: bill::ActiveModel = model.clone().into();
        am.delete(db).await.map(|_| Some(model))
    } else {
        Ok(None)
    }
}
