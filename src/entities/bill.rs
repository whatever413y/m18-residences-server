use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::{tenant, electricity_reading, additional_charge};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "bill")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub reading_id: i32,
    pub tenant_id: i32,
    pub room_charges: i32,
    pub electric_charges: i32,
    pub total_amount: i32,
    pub receipt_url: Option<String>,
    pub paid: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "electricity_reading::Entity", from = "Column::ReadingId", to = "electricity_reading::Column::Id")]
    Reading,
    #[sea_orm(belongs_to = "tenant::Entity", from = "Column::TenantId", to = "tenant::Column::Id")]
    Tenant,
    #[sea_orm(has_many = "additional_charge::Entity")]
    AdditionalCharges,
}

impl Related<additional_charge::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdditionalCharges.def()
    }
}

impl Related<electricity_reading::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Reading.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
