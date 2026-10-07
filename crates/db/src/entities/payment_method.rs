use sea_orm::entity::prelude::*;
use serde::Serialize;

/// A way to pay (bank or e-wallet) with its optional QR image (`image_key`, an R2 key).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "payment_method")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub account_name: Option<String>,
    pub account_number: Option<String>,
    pub sort_order: i32,
    pub image_key: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
