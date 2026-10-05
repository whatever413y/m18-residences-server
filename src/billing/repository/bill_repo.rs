//! Queries on `bill`, plus the statements the services run atomically.
//! Newest first means `created_at DESC, id DESC`: SQLite timestamps have
//! second precision, so the id breaks ties.
use m18_residences_db::entities::bill;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QueryTrait, Select, Set, Statement,
};

fn newest_first(query: Select<bill::Entity>) -> Select<bill::Entity> {
    query
        .order_by_desc(bill::Column::CreatedAt)
        .order_by_desc(bill::Column::Id)
}

/// Every bill, newest first.
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<bill::Model>, DbErr> {
    newest_first(bill::Entity::find()).all(db).await
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

/// Whether a bill of `tenant_id` has the receipt `receipt_url` (bills can share one only when it was set by hand).
pub async fn receipt_in_use(
    db: &DatabaseConnection,
    tenant_id: i32,
    receipt_url: &str,
) -> Result<bool, DbErr> {
    Ok(bill::Entity::find()
        .filter(bill::Column::TenantId.eq(tenant_id))
        .filter(bill::Column::ReceiptUrl.eq(receipt_url))
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

/// UPDATE of only the bill `id`'s payment image (`None` clears it).
pub fn set_payment_statement(
    backend: DbBackend,
    id: i32,
    payment_url: Option<String>,
) -> Statement {
    update_statement(
        backend,
        id,
        bill::ActiveModel {
            payment_url: Set(payment_url),
            ..Default::default()
        },
    )
}

/// DELETE of the bill `id`.
pub fn delete_statement(backend: DbBackend, id: i32) -> Statement {
    bill::Entity::delete_many()
        .filter(bill::Column::Id.eq(id))
        .build(backend)
}
