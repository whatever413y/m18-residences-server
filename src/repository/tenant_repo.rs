use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use chrono::Utc;
use crate::{entities::tenant};

pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<tenant::Model>, DbErr> {
    let tenants = tenant::Entity::find()
        .order_by_asc(tenant::Column::Name)
        .all(db)
        .await;

    match &tenants {
        Ok(list) => println!("✅ get_all: fetched {} tenants", list.len()),
        Err(err) => eprintln!("❌ get_all: error fetching tenants: {:?}", err),
    }

    tenants
}

pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    let result = tenant::Entity::find_by_id(id).one(db).await;

    match &result {
        Ok(Some(tenant)) => println!("✅ get_by_id: found tenant id={} name={}", tenant.id, tenant.name),
        Ok(None) => println!("⚠️ get_by_id: tenant id={} not found", id),
        Err(err) => eprintln!("❌ get_by_id: error fetching tenant id={}: {:?}", id, err),
    }

    result
}

pub async fn get_by_name(db: &DatabaseConnection, name: &str) -> Result<Option<tenant::Model>, DbErr> {
    let result = tenant::Entity::find()
        .filter(tenant::Column::Name.eq(name))
        .one(db)
        .await;

    match &result {
        Ok(Some(tenant)) => println!("✅ get_by_name: found tenant id={} name={}", tenant.id, tenant.name),
        Ok(None) => println!("⚠️ get_by_name: tenant with name '{}' not found", name),
        Err(err) => eprintln!("❌ get_by_name: error fetching tenant by name '{}': {:?}", name, err),
    }

    result
}

pub async fn create(db: &DatabaseConnection, mut item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    let now = Utc::now().naive_utc();
    item.created_at = Set(now);
    item.updated_at = Set(now);

    let result = item.insert(db).await;

    match &result {
        Ok(tenant) => println!("✅ create: created tenant id={} name={}", tenant.id, tenant.name),
        Err(err) => eprintln!("❌ create: error creating tenant: {:?}", err),
    }

    result
}

pub async fn update(db: &DatabaseConnection, id: i32, mut item: tenant::ActiveModel) -> Result<tenant::Model, DbErr> {
    item.id = Set(id);
    item.updated_at = Set(Utc::now().naive_utc());

    let result = item.update(db).await;

    match &result {
        Ok(tenant) => println!("✅ update: updated tenant id={} name={}", tenant.id, tenant.name),
        Err(err) => eprintln!("❌ update: error updating tenant id={}: {:?}", id, err),
    }

    result
}

pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    if let Some(model) = tenant::Entity::find_by_id(id).one(db).await? {
        let am: tenant::ActiveModel = model.clone().into();
        let res = am.delete(db).await;

        match &res {
            Ok(_) => println!("✅ delete: deleted tenant id={} name={}", model.id, model.name),
            Err(err) => eprintln!("❌ delete: error deleting tenant id={}: {:?}", id, err),
        }

        res.map(|_| Some(model))
    } else {
        println!("⚠️ delete: tenant id={} not found", id);
        Ok(None)
    }
}
