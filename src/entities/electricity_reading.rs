use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::{tenant, room};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "Electricity_Readings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub tenant_id: i32,
    pub room_id: i32,
    pub prev_reading: i32,
    pub curr_reading: i32,
    pub consumption: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "tenant::Entity", from = "Column::TenantId", to = "tenant::Column::Id")]
    Tenant,
    #[sea_orm(belongs_to = "room::Entity", from = "Column::RoomId", to = "room::Column::Id")]
    Room,
}

impl Related<tenant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tenant.def()
    }
}

impl Related<room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Room.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
