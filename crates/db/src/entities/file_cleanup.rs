use sea_orm::entity::prelude::*;

/// A file whose archiving failed, retried by the daily scheduled run (migration 0007).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "file_cleanup")]
pub struct Model {
    /// The file's R2 key (still live, to be moved to `archive/<key>`).
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    /// What the file was, for the logs (e.g. "receipt of bill 7").
    pub reason: String,
    /// Retries so far.
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
