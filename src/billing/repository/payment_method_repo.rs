use m18_residences_db::entities::payment_method;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder, QuerySelect, Set,
};

/// Every payment method in the order tenants see them: by `sort_order`, then by id.
pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<payment_method::Model>, DbErr> {
    payment_method::Entity::find()
        .order_by_asc(payment_method::Column::SortOrder)
        .order_by_asc(payment_method::Column::Id)
        .all(db)
        .await
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<payment_method::Model>, DbErr> {
    payment_method::Entity::find_by_id(id).one(db).await
}

/// One past the largest `sort_order` (1 when there is none): where a new method goes by default.
pub async fn next_sort_order(db: &DatabaseConnection) -> Result<i32, DbErr> {
    let last = payment_method::Entity::find()
        .order_by_desc(payment_method::Column::SortOrder)
        .limit(1)
        .one(db)
        .await?;
    Ok(last.map_or(1, |m| m.sort_order + 1))
}

/// INSERT … RETURNING.
pub async fn create(
    db: &DatabaseConnection,
    item: payment_method::ActiveModel,
) -> Result<payment_method::Model, DbErr> {
    item.insert(db).await
}

/// UPDATE … RETURNING of the set columns; `DbErr::RecordNotUpdated` if `id` doesn't exist.
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    mut item: payment_method::ActiveModel,
) -> Result<payment_method::Model, DbErr> {
    item.id = Set(id);
    item.update(db).await
}

/// Points the method at another QR image (`None` = no image); `DbErr::RecordNotUpdated` if `id` doesn't exist.
pub async fn set_image_key(
    db: &DatabaseConnection,
    id: i32,
    image_key: Option<String>,
) -> Result<payment_method::Model, DbErr> {
    let item = payment_method::ActiveModel {
        image_key: Set(image_key),
        ..Default::default()
    };
    update(db, id, item).await
}

/// DELETE … RETURNING in one statement: the deleted method, or `None` if there was none.
pub async fn delete(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<payment_method::Model>, DbErr> {
    payment_method::Entity::delete_by_id(id)
        .exec_with_returning(db)
        .await
}
