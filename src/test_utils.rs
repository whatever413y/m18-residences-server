use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use std::env;

#[cfg(test)]
fn load_env() {
    dotenvy::from_filename(".env.test").ok();
}

#[cfg(test)]
pub async fn get_test_db() -> DatabaseConnection {
    load_env();

    let url = env::var("TEST_DATABASE_URL").expect("❌ TEST_DATABASE_URL must be set in .env.test");

    Database::connect(&url)
        .await
        .expect("❌ Failed to connect to test DB")
}

/// Reset a specific table
#[cfg(test)]
pub async fn reset_table(db: &DatabaseConnection, table_name: &str) {
    let sql = format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE;", table_name);
    db.execute(Statement::from_string(db.get_database_backend(), sql))
        .await
        .unwrap();
}
