use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, DbErr, QueryOrder};
use crate::entities::room;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<room::Model>, DbErr> {
    room::Entity::find()
        .order_by_asc(room::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    room::Entity::find_by_id(id).one(db).await
}

pub async fn create(db: &DatabaseConnection, item: room::ActiveModel) -> Result<room::Model, DbErr> {
    item.insert(db).await
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: room::ActiveModel) -> Result<room::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<room::Model>, DbErr> {
    if let Some(model) = room::Entity::find_by_id(id).one(db).await? {
        let am: room::ActiveModel = model.clone().into();
        am.delete(db).await.map(|_| Some(model))
    } else {
        Ok(None)
    }
}

// ---------------------- INLINE TESTS ----------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{get_test_db, reset_table};    
    #[tokio::test]
    async fn test_create_and_get_room() {
        let db = get_test_db().await;
        reset_table(&db, "room").await;

        let new_room = room::ActiveModel {
            name: Set("Unit Test Room".into()),
            rent: Set(1000),
            ..Default::default()
        };

        // Create room
        let created = create(&db, new_room).await.unwrap();
        assert_eq!(created.name, "Unit Test Room");

        // Fetch by id
        let fetched = get_by_id(&db, created.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, created.name);
    }

    #[tokio::test]
    async fn test_update_room() {
        let db = get_test_db().await;
        reset_table(&db, "room").await;

        let new_room = room::ActiveModel {
            name: Set("Old Name".into()),
            rent: Set(500),
            ..Default::default()
        };

        let created = create(&db, new_room).await.unwrap();

        let mut updated_room: room::ActiveModel = created.clone().into();
        updated_room.name = Set("New Name".into());
        updated_room.rent = Set(1500);

        let updated = update(&db, created.id, updated_room).await.unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.rent, 1500);
    }

    #[tokio::test]
    async fn test_delete_room() {
        let db = get_test_db().await;
        reset_table(&db, "room").await;

        let new_room = room::ActiveModel {
            name: Set("Delete Me".into()),
            rent: Set(100),
            ..Default::default()
        };

        let created = create(&db, new_room).await.unwrap();
        let deleted = delete(&db, created.id).await.unwrap();
        assert!(deleted.is_some());

        let should_be_none = get_by_id(&db, created.id).await.unwrap();
        assert!(should_be_none.is_none());
    }
}
