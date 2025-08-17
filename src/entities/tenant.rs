use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::{room, electricity_reading};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)] 
#[sea_orm(table_name = "Tenants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub room_id: i32,
    pub name: String,
    pub is_active: bool,
    pub join_date: DateTimeUtc,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "room::Entity", from = "Column::RoomId", to = "room::Column::Id")]
    Room,
    #[sea_orm(has_many = "electricity_reading::Entity")]
    Readings,
}

impl Related<room::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Room.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
