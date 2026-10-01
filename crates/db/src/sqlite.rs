//! Native SQLite for tests: a fresh in-memory database per call, with the D1
//! migrations applied, so every test is isolated and tests can run in parallel.
use std::{sync::Arc, time::Duration};

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement,
    TransactionTrait,
};

use crate::{Atomic, Db, schema::MIGRATIONS};

/// An empty database with the current schema.
pub async fn connect_in_memory() -> Result<Db, DbErr> {
    let mut options = ConnectOptions::new("sqlite::memory:");
    // Every connection to `sqlite::memory:` is its own database: keep exactly one, forever.
    options
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(24 * 60 * 60))
        .max_lifetime(Duration::from_secs(24 * 60 * 60))
        .sqlx_logging(false);
    let conn = Database::connect(options).await?;
    // D1 enforces foreign keys; make sure this connection does too.
    conn.execute_unprepared("PRAGMA foreign_keys = ON").await?;
    for (_, sql) in MIGRATIONS {
        conn.execute_unprepared(sql).await?;
    }
    Ok(Db::new(conn.clone(), Arc::new(SqliteAtomic(conn))))
}

/// [`Atomic`] as one SQLite transaction; dropping it on error rolls back.
struct SqliteAtomic(DatabaseConnection);

#[async_trait::async_trait]
impl Atomic for SqliteAtomic {
    async fn run(&self, statements: Vec<Statement>) -> Result<(), DbErr> {
        let txn = self.0.begin().await?;
        for statement in statements {
            txn.execute_raw(statement).await?;
        }
        txn.commit().await
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    use super::connect_in_memory;

    #[tokio::test]
    async fn applies_the_schema_and_enforces_foreign_keys() {
        let db = connect_in_memory().await.unwrap();
        let fk = db
            .conn()
            .execute_unprepared("INSERT INTO tenant (room_id, name) VALUES (999, 'nobody')")
            .await
            .expect_err("a tenant without its room must be refused");
        assert!(
            fk.to_string().contains("FOREIGN KEY constraint failed"),
            "{fk}"
        );
    }

    #[tokio::test]
    async fn atomic_rolls_back_everything_on_failure() {
        let db = connect_in_memory().await.unwrap();
        let insert = |name: &str| {
            Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO room (name, rent) VALUES (?, 1)",
                [name.into()],
            )
        };
        let err = db
            .atomic(vec![insert("A"), insert("A")])
            .await
            .expect_err("duplicate name");
        assert!(
            err.to_string().contains("UNIQUE constraint failed"),
            "{err}"
        );
        let rows = db
            .conn()
            .query_all_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT id FROM room",
            ))
            .await
            .unwrap();
        assert!(
            rows.is_empty(),
            "the first insert must have been rolled back"
        );

        db.atomic(vec![insert("A"), insert("B")]).await.unwrap();
        let rows = db
            .conn()
            .query_all_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT id FROM room",
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
    }
}
