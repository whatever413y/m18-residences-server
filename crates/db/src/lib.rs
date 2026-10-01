//! M18 Residences data layer: every entity, the D1 (SQLite) migrations and the
//! database handle the domains use.
//!
//! The migrations in `migrations/` are the only source of DDL. Real databases
//! get them from `wrangler d1 migrations apply`; the native tests apply the same
//! files to an in-memory SQLite database (`sqlite` feature).
pub mod entities;
pub mod rows;
pub mod schema;

#[cfg(target_arch = "wasm32")]
pub mod d1;
#[cfg(feature = "sqlite")]
pub mod sqlite;

use std::sync::Arc;

use sea_orm::{DatabaseConnection, DbBackend, DbErr, Statement};

/// Runs statements atomically: all of them or none. Production uses one D1
/// batch, the native tests a transaction. D1 has no interactive transactions,
/// so every multi-statement write must be expressed as such a list.
#[async_trait::async_trait]
pub trait Atomic: Send + Sync {
    async fn run(&self, statements: Vec<Statement>) -> Result<(), DbErr>;
}

/// The database handle the domains share: a SeaORM connection for queries and
/// single-statement writes, plus [`Db::atomic`] for multi-statement writes.
#[derive(Clone)]
pub struct Db {
    conn: DatabaseConnection,
    atomic: Arc<dyn Atomic>,
}

impl Db {
    pub fn new(conn: DatabaseConnection, atomic: Arc<dyn Atomic>) -> Self {
        Self { conn, atomic }
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    /// The SQL dialect to build statements for [`Db::atomic`] with.
    pub fn backend(&self) -> DbBackend {
        self.conn.get_database_backend()
    }

    /// Runs `statements` as one atomic unit (see [`Atomic`]).
    pub async fn atomic(&self, statements: Vec<Statement>) -> Result<(), DbErr> {
        if statements.is_empty() {
            return Ok(());
        }
        self.atomic.run(statements).await
    }
}
