use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

pub async fn run_migrations(url: &str) {
    let db = Database::connect(url)
        .await
        .expect("Failed to connect for migrations");
    Migrator::up(&db, None).await.expect("Failed to run migrations");
}

pub async fn connect() -> Result<DatabaseConnection, sea_orm::DbErr> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Database::connect(&database_url).await
}