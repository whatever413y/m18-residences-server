//! Queries on `bill`, plus the statements the services run atomically.
//! Newest first means `created_at DESC, id DESC`: SQLite timestamps have
//! second precision, so the id breaks ties.
use chrono::{NaiveDate, NaiveTime};
use m18_residences_db::entities::bill;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, Select, Set, Statement, sea_query::Expr,
};

use crate::billing::repository::reading_read_repo;

fn newest_first(query: Select<bill::Entity>) -> Select<bill::Entity> {
    query
        .order_by_desc(bill::Column::CreatedAt)
        .order_by_desc(bill::Column::Id)
}

/// Which bills [`get_filtered`] returns; every filter that is set must match.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BillFilter {
    /// Created on or after this day, **or** without a receipt (every open bill).
    pub since: Option<NaiveDate>,
    /// Created in this year.
    pub year: Option<i32>,
    pub tenant_id: Option<i32>,
    /// Their reading is in this room.
    pub room_id: Option<i32>,
}

/// `YYYY-MM-DD 00:00:00`, as `created_at` is stored.
fn start_of(day: NaiveDate) -> chrono::NaiveDateTime {
    day.and_time(NaiveTime::MIN)
}

/// No receipt yet (NULL or empty).
fn open() -> Condition {
    Condition::any()
        .add(bill::Column::ReceiptUrl.is_null())
        .add(bill::Column::ReceiptUrl.eq(""))
}

/// The bills matching `filter`, newest first.
pub async fn get_filtered(
    db: &DatabaseConnection,
    filter: &BillFilter,
) -> Result<Vec<bill::Model>, DbErr> {
    let mut query = bill::Entity::find();
    if let Some(since) = filter.since {
        query = query.filter(
            Condition::any()
                .add(bill::Column::CreatedAt.gte(start_of(since)))
                .add(open()),
        );
    }
    if let Some(year) = filter.year {
        // A year the calendar can't hold matches nothing.
        let (Some(first), Some(next)) = (
            NaiveDate::from_ymd_opt(year, 1, 1),
            NaiveDate::from_ymd_opt(year + 1, 1, 1),
        ) else {
            return Ok(Vec::new());
        };
        query = query
            .filter(bill::Column::CreatedAt.gte(start_of(first)))
            .filter(bill::Column::CreatedAt.lt(start_of(next)));
    }
    if let Some(tenant_id) = filter.tenant_id {
        query = query.filter(bill::Column::TenantId.eq(tenant_id));
    }
    if let Some(room_id) = filter.room_id {
        query = query.filter(
            bill::Column::ReadingId.in_subquery(reading_read_repo::ids_in_room_query(room_id)),
        );
    }
    newest_first(query).all(db).await
}

/// The years bills were created in, newest first.
pub async fn get_years(db: &DatabaseConnection) -> Result<Vec<i32>, DbErr> {
    let years: Vec<i64> = bill::Entity::find()
        .select_only()
        .column_as(
            Expr::cust("CAST(substr(\"bill\".\"created_at\", 1, 4) AS INTEGER)"),
            "year",
        )
        .distinct()
        .order_by_desc(Expr::cust("1"))
        .into_tuple()
        .all(db)
        .await?;
    years
        .into_iter()
        .map(|y| i32::try_from(y).map_err(|_| DbErr::Custom(format!("year {y} out of range"))))
        .collect()
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<bill::Model>, DbErr> {
    bill::Entity::find_by_id(id).one(db).await
}

/// The bill of a reading (there is at most one).
pub async fn get_by_reading_id(
    db: &DatabaseConnection,
    reading_id: i32,
) -> Result<Option<bill::Model>, DbErr> {
    bill::Entity::find()
        .filter(bill::Column::ReadingId.eq(reading_id))
        .one(db)
        .await
}

/// A tenant's newest bill.
pub async fn get_latest_by_tenant_id(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Option<bill::Model>, DbErr> {
    newest_first(bill::Entity::find().filter(bill::Column::TenantId.eq(tenant_id)))
        .one(db)
        .await
}

/// A tenant's bills, newest first.
pub async fn get_all_by_tenant_id(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<bill::Model>, DbErr> {
    newest_first(bill::Entity::find().filter(bill::Column::TenantId.eq(tenant_id)))
        .all(db)
        .await
}

/// Whether a bill has the receipt stored at `key` (bills could share one only when it was set by hand,
/// before receipts could only be kept or cleared that way).
pub async fn receipt_in_use(db: &DatabaseConnection, key: &str) -> Result<bool, DbErr> {
    Ok(bill::Entity::find()
        .filter(bill::Column::ReceiptKey.eq(key))
        .one(db)
        .await?
        .is_some())
}

/// INSERT of a new bill (its id and timestamps come from the column defaults).
pub fn insert_statement(backend: DbBackend, bill: bill::ActiveModel) -> Statement {
    bill::Entity::insert(bill).build(backend)
}

/// UPDATE of the bill `id`: only the columns `Set` in `bill` change.
pub fn update_statement(backend: DbBackend, id: i32, bill: bill::ActiveModel) -> Statement {
    bill::Entity::update_many()
        .set(bill)
        .filter(bill::Column::Id.eq(id))
        .build(backend)
}

/// [`update_statement`], applied only while the bill's receipt is still `receipt_url` (`IS`: NULL matches NULL).
pub fn update_if_receipt_statement(
    backend: DbBackend,
    id: i32,
    bill: bill::ActiveModel,
    receipt_url: Option<&str>,
) -> Statement {
    bill::Entity::update_many()
        .set(bill)
        .filter(bill::Column::Id.eq(id))
        .filter(Expr::cust_with_values(
            "\"bill\".\"receipt_url\" IS ?",
            [receipt_url.map(str::to_string)],
        ))
        .build(backend)
}

/// Sets only the bill `id`'s payment image (its file name and key; `None` clears it), in one
/// statement; with `only_without_receipt`, only while the bill has no receipt. The number of bills
/// changed: 0 when there is no such bill (or it has a receipt by then).
pub async fn set_payment(
    db: &DatabaseConnection,
    id: i32,
    payment: Option<(String, String)>,
    only_without_receipt: bool,
) -> Result<u64, DbErr> {
    let (payment_url, payment_key) = payment.unzip();
    let mut update = bill::Entity::update_many()
        .set(bill::ActiveModel {
            payment_url: Set(payment_url),
            payment_key: Set(payment_key),
            ..Default::default()
        })
        .filter(bill::Column::Id.eq(id));
    if only_without_receipt {
        update = update.filter(open());
    }
    Ok(update.exec(db).await?.rows_affected)
}

/// DELETE of the bill `id`.
pub fn delete_statement(backend: DbBackend, id: i32) -> Statement {
    bill::Entity::delete_many()
        .filter(bill::Column::Id.eq(id))
        .build(backend)
}
