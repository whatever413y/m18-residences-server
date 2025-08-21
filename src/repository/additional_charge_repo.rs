use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};
use crate::entities::additional_charge;

#[allow(dead_code)]
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<additional_charge::Model>, DbErr> {
    additional_charge::Entity::find()
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn get_all_by_bill_id<C>(conn: &C, bill_id: i32) -> Result<Vec<additional_charge::Model>, DbErr> where C: ConnectionTrait + TransactionTrait {
    additional_charge::Entity::find()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .order_by_asc(additional_charge::Column::CreatedAt)
        .all(conn)
        .await
}

#[allow(dead_code)]
pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<additional_charge::Model>, DbErr> {
    additional_charge::Entity::find_by_id(id).one(db).await
}

pub async fn create(db: &DatabaseTransaction, item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    item.insert(db).await
}

#[allow(dead_code)]
pub async fn update(db: &DatabaseConnection, item: additional_charge::ActiveModel) -> Result<additional_charge::Model, DbErr> {
    item.update(db).await
}

#[allow(dead_code)]
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::Id.eq(id))
        .exec(db)
        .await
        .map(|_| ())
}

pub async fn delete_many_by_bill_id<C>(
    conn: &C,
    bill_id: i32,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let res = additional_charge::Entity::delete_many()
        .filter(additional_charge::Column::BillId.eq(bill_id))
        .exec(conn)
        .await?;

    Ok(res.rows_affected)
}

// ---------------------- INLINE TESTS ----------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{get_test_db, reset_table};
    use crate::entities::{room, tenant, electricity_reading, bill, additional_charge};
    use sea_orm::{ActiveModelTrait, Set};
    use chrono::Utc;

    /// Reset all related tables for a clean test run
    async fn reset_tables_for_test(db: &DatabaseConnection) {
        reset_table(db, "additional_charge").await;
        reset_table(db, "bill").await;
        reset_table(db, "electricity_reading").await;
        reset_table(db, "tenant").await;
        reset_table(db, "room").await;
    }

    /// Creates a Bill with dependencies (Room, Tenant, Reading)
    async fn setup_bill(db: &DatabaseConnection) -> bill::Model {
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

        // Create Electricity Reading
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

        // Create Bill
        bill::ActiveModel {
            reading_id: Set(reading.id),
            tenant_id: Set(tenant.id),
            room_charges: Set(2000),
            electric_charges: Set(1000),
            total_amount: Set(3000),
            receipt_url: Set(None),
            paid: Set(false),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_additional_charge() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let bill = setup_bill(&db).await;

        let txn = db.begin().await.unwrap();
        let charge = create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(500),
                description: Set("Late fee".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();

        let fetched = get_by_id(&db, charge.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().description, "Late fee");
    }

    #[tokio::test]
    async fn test_get_all_and_get_all_by_bill_id() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let bill = setup_bill(&db).await;

        let txn = db.begin().await.unwrap();
        create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(200),
                description: Set("Water".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(300),
                description: Set("Maintenance".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();

        let all = get_all(&db).await.unwrap();
        assert_eq!(all.len(), 2);

        let by_bill = get_all_by_bill_id(&db, bill.id).await.unwrap();
        assert_eq!(by_bill.len(), 2);
        assert!(by_bill.iter().any(|c| c.description == "Water"));
    }

    #[tokio::test]
    async fn test_update_additional_charge() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let bill = setup_bill(&db).await;

        let txn = db.begin().await.unwrap();
        let charge = create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(100),
                description: Set("Old desc".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();

        let mut am: additional_charge::ActiveModel = charge.into();
        am.description = Set("Updated desc".to_string());
        let updated = update(&db, am).await.unwrap();

        assert_eq!(updated.description, "Updated desc");
    }

    #[tokio::test]
    async fn test_delete_and_delete_many_by_bill_id() {
        let db = get_test_db().await;
        reset_tables_for_test(&db).await;

        let bill = setup_bill(&db).await;

        let txn = db.begin().await.unwrap();
        let charge1 = create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(100),
                description: Set("Charge A".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        create(
            &txn,
            additional_charge::ActiveModel {
                bill_id: Set(bill.id),
                amount: Set(200),
                description: Set("Charge B".to_string()),
                created_at: Set(Utc::now().naive_utc()),
                updated_at: Set(Utc::now().naive_utc()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        txn.commit().await.unwrap();

        // delete single
        delete(&db, charge1.id).await.unwrap();
        let fetched = get_by_id(&db, charge1.id).await.unwrap();
        assert!(fetched.is_none());

        // delete many by bill_id
        let rows = delete_many_by_bill_id(&db, bill.id).await.unwrap();
        assert!(rows > 0);

        let by_bill = get_all_by_bill_id(&db, bill.id).await.unwrap();
        assert!(by_bill.is_empty());
    }
}
