//! Queries on `file_cleanup`: files whose archiving failed, retried by the daily scheduled run.
use m18_residences_db::entities::file_cleanup;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Statement,
    sea_query::{Expr, ExprTrait},
};

/// Records `key` to retry; a key already recorded keeps its row.
pub async fn record(
    db: &DatabaseConnection,
    key: &str,
    reason: &str,
    error: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO file_cleanup (key, reason, last_error) VALUES (?, ?, ?) ON CONFLICT (key) DO NOTHING",
        [key.into(), reason.into(), error.into()],
    ))
    .await
    .map(|_| ())
}

/// Up to `limit` recorded files, oldest first.
pub async fn oldest(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<Vec<file_cleanup::Model>, DbErr> {
    file_cleanup::Entity::find()
        .order_by_asc(file_cleanup::Column::CreatedAt)
        .order_by_asc(file_cleanup::Column::Key)
        .limit(limit)
        .all(db)
        .await
}

/// Forgets `key` (archived, or already gone).
pub async fn remove(db: &DatabaseConnection, key: &str) -> Result<(), DbErr> {
    file_cleanup::Entity::delete_many()
        .filter(file_cleanup::Column::Key.eq(key))
        .exec(db)
        .await
        .map(|_| ())
}

/// Counts one more failed retry of `key`.
pub async fn retry_failed(db: &DatabaseConnection, key: &str, error: &str) -> Result<(), DbErr> {
    file_cleanup::Entity::update_many()
        .col_expr(
            file_cleanup::Column::Attempts,
            Expr::col(file_cleanup::Column::Attempts).add(1),
        )
        .col_expr(file_cleanup::Column::LastError, Expr::value(error))
        .filter(file_cleanup::Column::Key.eq(key))
        .exec(db)
        .await
        .map(|_| ())
}
