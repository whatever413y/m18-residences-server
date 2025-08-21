use sea_orm::Database;
use sea_orm::DatabaseConnection;
use migration::{Migrator, MigratorTrait, sea_orm::Database as MigrationDatabase, sea_orm::DatabaseConnection as MigrationDatabaseConnection};

pub async fn run_migrations(url: &str) {
    let db: MigrationDatabaseConnection = MigrationDatabase::connect(url)
        .await
        .expect("Failed to connect for migrations");
    Migrator::up(&db, None).await.expect("Failed to run migrations");
}

pub async fn connect() -> Result<DatabaseConnection, sea_orm::DbErr> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Database::connect(&database_url).await
}