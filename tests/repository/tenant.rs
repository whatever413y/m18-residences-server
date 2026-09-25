use m18_residences_server::entities::{room, tenant};
use m18_residences_server::repository::room_repo;
use m18_residences_server::repository::tenant_repo::*;
use crate::common::{get_test_db, reset_table};
use sea_orm::{DatabaseConnection, Set};
use chrono::Utc;

/// Starts from empty tenant/room tables and creates the room the test tenants
/// live in (tenant.room_id references room.id).
async fn setup_room(db: &DatabaseConnection) -> room::Model {
    reset_table(db, "tenant").await;
    reset_table(db, "room").await;

    room_repo::create(
        db,
        room::ActiveModel {
            name: Set("Test Room".into()),
            rent: Set(1000),
            ..Default::default()
        }
    ).await.unwrap()
}

fn new_tenant_model(room_id: i32, name: &str) -> tenant::ActiveModel {
    tenant::ActiveModel {
        room_id: Set(room_id),
        name: Set(name.to_string()),
        is_active: Set(true),
        join_date: Set(Utc::now().naive_utc()),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_create_and_get_tenant() {
    let db = get_test_db().await;
    let room = setup_room(&db).await;

    let new_tenant = new_tenant_model(room.id, "John Doe");

    // Create tenant
    let created = create(&db, new_tenant).await.unwrap();
    assert_eq!(created.name, "John Doe");

    // Fetch by id
    let fetched = get_by_id(&db, created.id).await.unwrap().unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, created.name);

    // Fetch by name
    let fetched_by_name = get_by_name(&db, "John Doe").await.unwrap().unwrap();
    assert_eq!(fetched_by_name.id, created.id);
}

#[tokio::test]
async fn test_update_tenant() {
    let db = get_test_db().await;
    let room = setup_room(&db).await;

    let new_tenant = new_tenant_model(room.id, "Jane Doe");
    let created = create(&db, new_tenant).await.unwrap();

    let mut updated_tenant: tenant::ActiveModel = created.clone().into();
    updated_tenant.name = Set("Jane Smith".into());

    let updated = update(&db, created.id, updated_tenant).await.unwrap();
    assert_eq!(updated.name, "Jane Smith");
}

#[tokio::test]
async fn test_delete_tenant() {
    let db = get_test_db().await;
    let room = setup_room(&db).await;

    let new_tenant = new_tenant_model(room.id, "Delete Me");
    let created = create(&db, new_tenant).await.unwrap();

    let deleted = delete(&db, created.id).await.unwrap();
    assert!(deleted.is_some());

    let should_be_none = get_by_id(&db, created.id).await.unwrap();
    assert!(should_be_none.is_none());
}
