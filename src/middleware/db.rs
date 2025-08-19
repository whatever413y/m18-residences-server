use sea_orm::Database;
use sea_orm::DatabaseConnection;

pub async fn connect() -> Result<DatabaseConnection, sea_orm::DbErr> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Database::connect(&database_url).await
}
