use m18_residences_server::entities::electricity_reading;
use m18_residences_server::repository::electricity_reading_repo::*;
use m18_residences_server::entities::electricity_reading::ActiveModel;
use m18_residences_server::entities::{room, tenant};
use crate::common::{get_test_db, reset_table};
use m18_residences_server::repository::{room_repo, tenant_repo};
use sea_orm::Set;
use chrono::Utc;

async fn setup_room_and_tenant(db: &sea_orm::DatabaseConnection) -> (room::Model, tenant::Model) {
    reset_table(db, "electricity_reading").await;
    reset_table(db, "tenant").await;
    reset_table(db, "room").await;

    // Create a room
    let room = room_repo::create(
        db,
        room::ActiveModel {
            name: Set("Test Room".into()),
            rent: Set(1000),
            ..Default::default()
        }
    ).await.unwrap();

    // Create a tenant in that room
    let tenant = tenant_repo::create(
        db,
        tenant::ActiveModel {
            name: Set("Test Tenant".into()),
            room_id: Set(room.id),
            is_active: Set(true),
            join_date: Set(Utc::now().naive_utc()),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
    ).await.unwrap();

    (room, tenant)
}

fn new_reading_model(tenant_id: i32, room_id: i32, prev: i32, curr: i32) -> electricity_reading::ActiveModel {
    electricity_reading::ActiveModel {
        tenant_id: Set(tenant_id),
        room_id: Set(room_id),
        prev_reading: Set(prev),
        curr_reading: Set(curr),
        consumption: Set(curr - prev),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_create_and_get_reading() {
    let db = get_test_db().await;
    let (room, tenant) = setup_room_and_tenant(&db).await;

    let reading = new_reading_model(tenant.id, room.id, 100, 150);
    let created = create(&db, reading).await.unwrap();

    let fetched = get_by_id(&db, created.id).await.unwrap().unwrap();
    assert_eq!(fetched.consumption, 50);
    assert_eq!(fetched.tenant_id, tenant.id);
    assert_eq!(fetched.room_id, room.id);
}

#[tokio::test]
async fn test_update_reading() {
    let db = get_test_db().await;
    let (room, tenant) = setup_room_and_tenant(&db).await;

    let reading = create(&db, new_reading_model(tenant.id, room.id, 100, 150))
        .await.unwrap();

    let mut updated: ActiveModel = reading.clone().into();
    updated.curr_reading = Set(200);
    updated.consumption = Set(200 - reading.prev_reading);

    let result = update(&db, updated).await.unwrap();
    assert_eq!(result.curr_reading, 200);
    assert_eq!(result.consumption, 100);
}

#[tokio::test]
async fn test_delete_reading() {
    let db = get_test_db().await;
    let (room, tenant) = setup_room_and_tenant(&db).await;

    let reading = create(&db, new_reading_model(tenant.id, room.id, 100, 150))
        .await.unwrap();

    let deleted = delete(&db, reading.id).await.unwrap();
    assert!(deleted.is_some());

    let should_be_none = get_by_id(&db, reading.id).await.unwrap();
    assert!(should_be_none.is_none());
}
