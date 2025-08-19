use sea_orm::entity::prelude::*;
use crate::entities::{tenant, electricity_reading};
use serde::Serialize; 

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)] 
#[sea_orm(table_name = "Rooms")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub rent: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "tenant::Entity")]
    Tenants,
    #[sea_orm(has_many = "electricity_reading::Entity")]
    Readings,
}

impl ActiveModelBehavior for ActiveModel {}
