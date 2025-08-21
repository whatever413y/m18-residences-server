use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use crate::entities::tenant;

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .order_by_asc(tenant::Column::Name)
        .all(db)
        .await
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find_by_id(id).one(db).await
}

pub async fn get_by_name(db: &DatabaseConnection, name: &str) -> Result<Option<tenant::Model>, DbErr> {
    tenant::Entity::find()
        .filter(tenant::Column::Name.eq(name))
        .one(db)
        .await
}

pub async fn create(db: &DatabaseConnection, item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    item.insert(db).await
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    if let Some(model) = tenant::Entity::find_by_id(id).one(db).await? {
        let am: tenant::ActiveModel = model.clone().into();
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
    use sea_orm::Set;
    use chrono::Utc;

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
        reset_table(&db, "tenant").await;

        let new_tenant = new_tenant_model(1, "John Doe");

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
        reset_table(&db, "tenant").await;

        let new_tenant = new_tenant_model(1, "Jane Doe");
        let created = create(&db, new_tenant).await.unwrap();

        let mut updated_tenant: tenant::ActiveModel = created.clone().into();
        updated_tenant.name = Set("Jane Smith".into());

        let updated = update(&db, created.id, updated_tenant).await.unwrap();
        assert_eq!(updated.name, "Jane Smith");
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let db = get_test_db().await;
        reset_table(&db, "tenant").await;

        let new_tenant = new_tenant_model(1, "Delete Me");
        let created = create(&db, new_tenant).await.unwrap();

        let deleted = delete(&db, created.id).await.unwrap();
        assert!(deleted.is_some());

        let should_be_none = get_by_id(&db, created.id).await.unwrap();
        assert!(should_be_none.is_none());
    }
}
