use crate::entities::{bill, additional_charge, electricity_reading};
use crate::services::bill_service::{BillWithCharges,};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

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
