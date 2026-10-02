//! Queries on `additional_charge`, plus the statements the services run
//! atomically. Charges come oldest first (`created_at, id`).
use m18_residences_db::entities::additional_charge;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QueryTrait, Statement,
};

use crate::billing::repository::MAX_IDS_PER_QUERY;

/// The charges of all `bill_ids`, oldest first within each bill: one query
/// per [`MAX_IDS_PER_QUERY`] bills.
pub async fn get_all_by_bill_ids(
    db: &DatabaseConnection,
    bill_ids: &[i32],
) -> Result<Vec<additional_charge::Model>, DbErr> {
    let mut charges = Vec::new();
    for chunk in bill_ids.chunks(MAX_IDS_PER_QUERY) {
        let found = additional_charge::Entity::find()
            .filter(additional_charge::Column::BillId.is_in(chunk.iter().copied()))
            .order_by_asc(additional_charge::Column::CreatedAt)
            .order_by_asc(additional_charge::Column::Id)
            .all(db)
            .await?;
        charges.extend(found);
    }
    Ok(charges)
}

/// The charges of one bill, oldest first.
pub async fn get_all_by_bill_id(
    db: &DatabaseConnection,
    bill_id: i32,
) -> Result<Vec<additional_charge::Model>, DbErr> {
    get_all_by_bill_ids(db, &[bill_id]).await
}

/// INSERT of a charge on the bill `bill_id`.
pub fn insert_statement(
    backend: DbBackend,
    bill_id: i32,
    amount: i32,
    description: &str,
) -> Statement {
    Statement::from_sql_and_values(
        backend,
        "INSERT INTO additional_charge (bill_id, amount, description) VALUES (?, ?, ?)",
        [bill_id.into(), amount.into(), description.into()],
    )
}

/// INSERT of a charge on the bill of `reading_id`, for a bill created in the
/// same atomic batch (its id is not known yet).
pub fn insert_for_reading_statement(
    backend: DbBackend,
    reading_id: i32,
    amount: i32,
    description: &str,
) -> Statement {
    Statement::from_sql_and_values(
        backend,
        "INSERT INTO additional_charge (bill_id, amount, description) \
         VALUES ((SELECT id FROM bill WHERE reading_id = ?), ?, ?)",
        [reading_id.into(), amount.into(), description.into()],
    )
}

/// DELETE of every charge of the bill `bill_id`.
pub fn delete_by_bill_id_statement(backend: DbBackend, bill_id: i32) -> Statement {
    additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .build(backend)
}
