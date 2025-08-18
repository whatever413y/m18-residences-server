use crate::repository::tenant_repo;
use crate::entities::tenant;
use sea_orm::{DatabaseConnection, DbErr};


/// Get all tenants
pub async fn get_all_tenants(db: &DatabaseConnection) -> Result<Vec<tenant::Model>, DbErr> {
    let result = tenant_repo::get_all(db).await;

    if let Ok(list) = &result {
        println!("✅ get_all_tenants: fetched {} tenants", list.len());
    } else if let Err(err) = &result {
        eprintln!("❌ get_all_tenants: error fetching tenants: {:?}", err);
    }

    result
}

/// Get tenant by ID 
pub async fn get_tenant_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<tenant::Model>, DbErr> {
    let result = tenant_repo::get_by_id(db, id).await;

    match &result {
        Ok(Some(t)) => println!("✅ get_tenant_by_id: found tenant id={} name={}", t.id, t.name),
        Ok(None) => println!("⚠️ get_tenant_by_id: tenant id={} not found", id),
        Err(err) => eprintln!("❌ get_tenant_by_id: error fetching tenant id={}: {:?}", id, err),
    }

    result
}

/// Get tenant by name 
pub async fn get_tenant_by_name(
    db: &DatabaseConnection,
    name: &str,
) -> Result<Option<tenant::Model>, DbErr> {
    let result = tenant_repo::get_by_name(db, name).await;

    match &result {
        Ok(Some(t)) => println!("✅ get_tenant_by_name: found tenant id={} name={}", t.id, t.name),
        Ok(None) => println!("⚠️ get_tenant_by_name: tenant with name '{}' not found", name),
        Err(err) => eprintln!("❌ get_tenant_by_name: error fetching tenant by name '{}': {:?}", name, err),
    }

    result
}

/// Create tenant 
pub async fn create_tenant(
    db: &DatabaseConnection,
    item: tenant::ActiveModel,
) -> Result<tenant::Model, DbErr> {
    let result = tenant_repo::create(db, item).await;

    if let Ok(ref t) = result {
        println!("✅ create_tenant: created id={} name={}", t.id, t.name);
    } else if let Err(ref err) = result {
        eprintln!("❌ create_tenant: error creating tenant: {:?}", err);
    }

    result
}


/// Update tenant 
pub async fn update_tenant(
    db: &DatabaseConnection,
    id: i32,
    item: tenant::ActiveModel,
) -> Result<tenant::Model, DbErr> {
    let result = tenant_repo::update(db, id, item).await;
    if let Ok(t) = &result {
        println!("✅ update_tenant: updated tenant id={} name={}", t.id, t.name);
    } else if let Err(err) = &result {
        eprintln!("❌ update_tenant: error updating tenant id={}: {:?}", id, err);
    }

    result
}

/// Delete tenant 
pub async fn delete_tenant(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<tenant::Model>, DbErr> {
    let result = tenant_repo::delete(db, id).await;

    match &result {
        Ok(Some(t)) => println!("✅ delete_tenant: deleted tenant id={} name={}", t.id, t.name),
        Ok(None) => println!("⚠️ delete_tenant: tenant id={} not found", id),
        Err(err) => eprintln!("❌ delete_tenant: error deleting tenant id={}: {:?}", id, err),
    }

    result
}