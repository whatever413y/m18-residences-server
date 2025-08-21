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

// ---------------------- INLINE TESTS ----------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{get_test_db, reset_table};
    use crate::entities::{room, tenant, electricity_reading, bill};
    use sea_orm::{ActiveModelTrait, Set};
    use chrono::Utc;

    /// Reset all related tables for a clean test run
    async fn reset_tables_for_test(db: &DatabaseConnection) {
        reset_table(db, "bill").await;
        reset_table(db, "electricity_reading").await;
        reset_table(db, "tenant").await;
        reset_table(db, "room").await;
    }

    /// Creates a Room, Tenant, and Electricity Reading for bill testing
    async fn setup_dependencies(db: &DatabaseConnection) -> (room::Model, tenant::Model, electricity_reading::Model) {
        // Create Room
        let room = room::ActiveModel {
            name: Set(format!("Test Room {}", Utc::now().timestamp())),
            rent: Set(1000),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        // Create Tenant
        let tenant = tenant::ActiveModel {
            name: Set(format!("Test Tenant {}", Utc::now().timestamp())),
            room_id: Set(room.id),
            is_active: Set(true),
            join_date: Set(Utc::now().naive_utc()),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        // Create Electricity Reading (valid with prev + curr)
        let reading = electricity_reading::ActiveModel {
            tenant_id: Set(tenant.id),
            room_id: Set(room.id),
            prev_reading: Set(100),
            curr_reading: Set(200),
            consumption: Set(100),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        (room, tenant, reading)
    }

    #[tokio::test]
    async fn test_create_and_get_bill() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let (_room, tenant, reading) = setup_dependencies(&db).await;

        let bill = bill::ActiveModel {
            reading_id: Set(reading.id),
            tenant_id: Set(tenant.id),
            room_charges: Set(1000),
            electric_charges: Set(500),
            total_amount: Set(1500),
            receipt_url: Set(None),
            paid: Set(false),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let fetched = get_by_id(&db, bill.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().total_amount, 1500);
    }

    #[tokio::test]
    async fn test_get_latest_by_tenant_id() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let (_room, tenant, reading) = setup_dependencies(&db).await;

        let bill = bill::ActiveModel {
            reading_id: Set(reading.id),
            tenant_id: Set(tenant.id),
            room_charges: Set(2000),
            electric_charges: Set(1000),
            total_amount: Set(3000),
            receipt_url: Set(None),
            paid: Set(true),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let latest = get_latest_by_tenant_id(&db, tenant.id).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, bill.id);
    }

    #[tokio::test]
async fn test_get_all_by_tenant_id() {
    let db = get_test_db().await;
    reset_tables_for_test(&db).await;

    let (room, tenant, _) = setup_dependencies(&db).await;

    // Insert 2 bills, each with its own reading
    for i in 0..2 {
        let reading = electricity_reading::ActiveModel {
            tenant_id: Set(tenant.id),
            room_id: Set(room.id),
            prev_reading: Set(100 + i * 50),
            curr_reading: Set(150 + i * 50),
            consumption: Set(50),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let _ = bill::ActiveModel {
            reading_id: Set(reading.id),
            tenant_id: Set(tenant.id),
            room_charges: Set(1000 + i * 100),
            electric_charges: Set(500 + i * 50),
            total_amount: Set(1500 + i * 150),
            receipt_url: Set(None),
            paid: Set(false),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    let bills = get_all_by_tenant_id(&db, tenant.id).await.unwrap();
    assert_eq!(bills.len(), 2);
}


    #[tokio::test]
    async fn test_delete_bill() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let (_room, tenant, reading) = setup_dependencies(&db).await;

        let bill = bill::ActiveModel {
            reading_id: Set(reading.id),
            tenant_id: Set(tenant.id),
            room_charges: Set(800),
            electric_charges: Set(200),
            total_amount: Set(1000),
            receipt_url: Set(None),
            paid: Set(false),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let deleted = delete(&db, bill.id).await.unwrap();
        assert!(deleted.is_some());

        let fetched = get_by_id(&db, bill.id).await.unwrap();
        assert!(fetched.is_none());
    }
}
