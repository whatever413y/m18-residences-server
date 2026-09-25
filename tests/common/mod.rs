//! Helpers shared by the DB-backed test binaries (`tests/api.rs`,
//! `tests/repository/`). Each binary compiles this module separately and uses
//! only part of it, hence the `dead_code` allowance.
#![allow(dead_code)]

use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::env;

fn load_env() {
    dotenvy::from_filename(".env.test").ok();
}

pub async fn get_test_db() -> DatabaseConnection {
    load_env();

    let url = env::var("TEST_DATABASE_URL").expect("❌ TEST_DATABASE_URL must be set in .env.test");

    Database::connect(&url)
        .await
        .expect("❌ Failed to connect to test DB")
}

/// Reset a specific table
pub async fn reset_table(db: &DatabaseConnection, table_name: &str) {
    let sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE;", table_name);
    db.execute_unprepared(&sql).await.unwrap();
}
