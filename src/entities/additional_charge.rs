use sea_orm::entity::prelude::*;
use serde::Serialize;
use crate::entities::bill;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "additional_charge")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub bill_id: i32,
    pub amount: i32,
    pub description: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "bill::Entity", from = "Column::BillId", to = "bill::Column::Id")]
    Bill,
}

impl Related<bill::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bill.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
