use crate::entities::bill;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder
};

/// GET all bills
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<bill::Model>, DbErr> {
    bill::Entity::find()
        .order_by_desc(bill::Column::CreatedAt)
        .all(db)
        .await
}

/// GET bill by id
#[allow(dead_code)]
pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<bill::Model>, DbErr> {
    bill::Entity::find_by_id(id).one(db).await
}

// Get latest bill for tenant
pub async fn get_latest_by_tenant_id(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Option<bill::Model>, DbErr> {
    bill::Entity::find()
        .filter(bill::Column::TenantId.eq(tenant_id))
        .order_by_desc(bill::Column::CreatedAt)
        .one(db)
        .await
}

/// GET bills for a tenant (basic)
pub async fn get_all_by_tenant_id(
    db: &DatabaseConnection,
    tenant_id: i32,
) -> Result<Vec<bill::Model>, DbErr> {
    bill::Entity::find()
        .filter(bill::Column::TenantId.eq(tenant_id))
        .order_by_desc(bill::Column::CreatedAt)
        .all(db)
        .await
}

/// DELETE a bill by ID
pub async fn delete<C>(conn: &C, id: i32) -> Result<Option<bill::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if let Some(model) = bill::Entity::find_by_id(id).one(conn).await? {
        let am: bill::ActiveModel = model.clone().into();
        am.delete(conn).await?;
        Ok(Some(model))
    } else {
        Ok(None)
    }
}

