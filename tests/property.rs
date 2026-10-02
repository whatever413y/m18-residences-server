//! Property: rooms, tenants and electricity readings — the legacy API and
//! repository tests, plus permissions, validation and error mapping.
mod common;

use axum::http::{Method, StatusCode};
use chrono::{NaiveDate, Utc};
use common::{TestApp, seed_bill, seed_reading, seed_room, seed_tenant, test_app};
use m18_residences_db::entities::{bill, electricity_reading, room, tenant};
use m18_residences_server::property::repository::{
    electricity_reading_repo, room_repo, tenant_repo,
};
use m18_residences_server::property::services::{
    Violation, electricity_reading_service::calculate_consumption, violation,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};
use serde_json::{Value, json};

const ROOM_NAME: &str = "Room 101";
const ROOM_RENT: i32 = 5000;
const TENANT_NAME: &str = "Juan Dela Cruz";
const JOIN_DATE: &str = "2026-01-01T00:00:00";

const ROOM_KEYS: &[&str] = &["id", "name", "rent", "created_at", "updated_at"];
const TENANT_KEYS: &[&str] = &[
    "id",
    "room_id",
    "name",
    "is_active",
    "join_date",
    "created_at",
    "updated_at",
];
const READING_KEYS: &[&str] = &[
    "id",
    "tenant_id",
    "room_id",
    "prev_reading",
    "curr_reading",
    "consumption",
    "created_at",
    "updated_at",
];

// ---- helpers ----

fn assert_keys(value: &Value, keys: &[&str], what: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what}: expected a JSON object, got {value}"));
    for key in keys {
        assert!(
            object.contains_key(*key),
            "{what}: missing key `{key}` in {value}"
        );
    }
    assert_eq!(
        object.len(),
        keys.len(),
        "{what}: unexpected keys in {value}"
    );
}

fn id_of(value: &Value) -> i64 {
    value["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("missing numeric id in {value}"))
}

fn error(message: &str) -> Value {
    json!({ "error": message })
}

fn error_text(body: &Value) -> &str {
    body["error"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a JSON error, got {body}"))
}

async fn create_room(app: &TestApp, token: &str) -> Value {
    let (status, body) = app
        .post(
            "/api/rooms",
            Some(token),
            json!({ "name": ROOM_NAME, "rent": ROOM_RENT }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "create room: {body}");
    assert_keys(&body, ROOM_KEYS, "created room");
    body
}

async fn create_tenant(app: &TestApp, token: &str, room_id: i64) -> Value {
    let (status, body) = app
        .post(
            "/api/tenants",
            Some(token),
            json!({ "name": TENANT_NAME, "room_id": room_id, "join_date": JOIN_DATE }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "create tenant: {body}");
    assert_keys(&body, TENANT_KEYS, "created tenant");
    body
}

async fn create_reading(app: &TestApp, token: &str, tenant: &Value, room: &Value) -> Value {
    let (status, body) = app
        .post(
            "/api/electricity-readings",
            Some(token),
            json!({
                "tenant_id": id_of(tenant),
                "room_id": id_of(room),
                "prev_reading": 100,
                "curr_reading": 150,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "create reading: {body}");
    assert_keys(&body, READING_KEYS, "created reading");
    body
}

struct Flow {
    room: Value,
    tenant: Value,
    reading: Value,
}

async fn run_flow(app: &TestApp, token: &str) -> Flow {
    let room = create_room(app, token).await;
    let tenant = create_tenant(app, token, id_of(&room)).await;
    let reading = create_reading(app, token, &tenant, &room).await;
    Flow {
        room,
        tenant,
        reading,
    }
}

fn reading_model(t: &tenant::Model, prev: i32, curr: i32) -> electricity_reading::ActiveModel {
    electricity_reading::ActiveModel {
        tenant_id: Set(t.id),
        room_id: Set(t.room_id),
        prev_reading: Set(prev),
        curr_reading: Set(curr),
        consumption: Set(curr - prev),
        ..Default::default()
    }
}

// ---- legacy API tests (tests/legacy/api.rs) ----

#[tokio::test]
async fn protected_route_rejects_missing_or_invalid_token() {
    let app = test_app().await;
    let expected = error("Authentication required");
    assert_eq!(
        app.get("/api/rooms", None).await,
        (StatusCode::UNAUTHORIZED, expected.clone())
    );
    assert_eq!(
        app.get("/api/rooms", Some("not-a-jwt")).await,
        (StatusCode::UNAUTHORIZED, expected)
    );
}

#[tokio::test]
async fn admin_flow_returns_contract_shapes() {
    let app = test_app().await;
    let token = app.admin_token();
    let flow = run_flow(&app, &token).await;

    assert_eq!(flow.room["name"], ROOM_NAME);
    assert_eq!(flow.room["rent"], ROOM_RENT);
    assert_eq!(
        flow.room["created_at"], flow.room["updated_at"],
        "timestamps come from the DB defaults"
    );

    assert_eq!(flow.tenant["room_id"], flow.room["id"]);
    assert_eq!(flow.tenant["name"], TENANT_NAME);
    assert_eq!(
        flow.tenant["join_date"], JOIN_DATE,
        "join_date round-trips exactly"
    );
    assert_eq!(flow.tenant["is_active"], true, "is_active defaults to true");

    assert_eq!(flow.reading["tenant_id"], flow.tenant["id"]);
    assert_eq!(flow.reading["room_id"], flow.room["id"]);
    assert_eq!(flow.reading["prev_reading"], 100);
    assert_eq!(flow.reading["curr_reading"], 150);
    assert_eq!(flow.reading["consumption"], 50, "server-computed");

    // Lists deep-equal the created resources.
    assert_eq!(
        app.get("/api/rooms", Some(&token)).await,
        (StatusCode::OK, json!([flow.room]))
    );
    assert_eq!(
        app.get("/api/tenants", Some(&token)).await,
        (StatusCode::OK, json!([flow.tenant]))
    );
    assert_eq!(
        app.get("/api/electricity-readings", Some(&token)).await,
        (StatusCode::OK, json!([flow.reading]))
    );
}

#[tokio::test]
async fn parameterized_routes_resolve_and_update_or_delete() {
    let app = test_app().await;
    let admin = app.admin_token();
    let token = Some(admin.as_str());
    let flow = run_flow(&app, &admin).await;
    let (room_id, tenant_id, reading_id) =
        (id_of(&flow.room), id_of(&flow.tenant), id_of(&flow.reading));

    let tenant_by_name = format!("/api/tenants/tenant/{}", TENANT_NAME.replace(' ', "%20"));
    for (uri, expected) in [
        (format!("/api/rooms/{room_id}"), &flow.room),
        (format!("/api/tenants/{tenant_id}"), &flow.tenant),
        (tenant_by_name, &flow.tenant),
        (
            format!("/api/electricity-readings/{reading_id}"),
            &flow.reading,
        ),
    ] {
        let (status, body) = app.get(&uri, token).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}: {body}");
        assert_eq!(&body, expected, "GET {uri}");
    }

    let (status, room) = app
        .put(
            &format!("/api/rooms/{room_id}"),
            token,
            json!({ "name": "Room 102", "rent": 5500 }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{room}");
    assert_eq!(room["name"], "Room 102");
    assert_eq!(room["rent"], 5500);
    assert_eq!(room["id"], room_id);
    assert_eq!(room["created_at"], flow.room["created_at"]);
    assert_eq!(
        room["updated_at"], flow.room["updated_at"],
        "updated_at is not bumped"
    );

    let (status, tenant) = app
        .put(
            &format!("/api/tenants/{tenant_id}"),
            token,
            json!({ "name": TENANT_NAME, "room_id": room_id, "join_date": JOIN_DATE, "is_active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{tenant}");
    assert_eq!(tenant["is_active"], false);
    assert_eq!(tenant["updated_at"], flow.tenant["updated_at"]);

    let (status, reading) = app
        .put(
            &format!("/api/electricity-readings/{reading_id}"),
            token,
            json!({
                "tenant_id": tenant_id,
                "room_id": room_id,
                "prev_reading": 100,
                "curr_reading": 170,
                "consumption": 1,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{reading}");
    assert_eq!(
        reading["consumption"], 70,
        "recomputed; the sent value is ignored"
    );
    assert_eq!(reading["updated_at"], flow.reading["updated_at"]);

    for uri in [
        format!("/api/electricity-readings/{reading_id}"),
        format!("/api/tenants/{tenant_id}"),
        format!("/api/rooms/{room_id}"),
    ] {
        let (status, body) = app.delete(&uri, token).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "DELETE {uri}: {body}");
        assert_eq!(body, Value::Null, "DELETE {uri} has an empty body");
    }
    assert_eq!(
        app.get("/api/rooms", token).await,
        (StatusCode::OK, json!([]))
    );
}

// ---- legacy repository tests (tests/legacy/repository) ----

#[tokio::test]
async fn repo_create_and_get_room() {
    let app = test_app().await;
    let db = app.db();
    let created = room_repo::create(
        db,
        room::ActiveModel {
            name: Set("Unit Test Room".into()),
            rent: Set(1000),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(created.name, "Unit Test Room");

    let fetched = room_repo::get_by_id(db, created.id).await.unwrap().unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, created.name);
    assert_eq!(fetched, created, "RETURNING and SELECT agree");
}

#[tokio::test]
async fn repo_update_room() {
    let app = test_app().await;
    let db = app.db();
    let created = seed_room(db, "Old Name", 500).await;

    let mut am: room::ActiveModel = created.clone().into();
    am.name = Set("New Name".into());
    am.rent = Set(1500);
    let updated = room_repo::update(db, created.id, am).await.unwrap();
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.rent, 1500);

    let missing = room::ActiveModel {
        name: Set("Ghost".into()),
        rent: Set(1),
        ..Default::default()
    };
    assert_eq!(
        room_repo::update(db, 999, missing).await.unwrap_err(),
        DbErr::RecordNotUpdated
    );
}

#[tokio::test]
async fn repo_delete_room() {
    let app = test_app().await;
    let db = app.db();
    let created = seed_room(db, "Delete Me", 100).await;
    let deleted = room_repo::delete(db, created.id).await.unwrap();
    assert_eq!(deleted, Some(created.clone()));
    assert!(
        room_repo::get_by_id(db, created.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(room_repo::delete(db, created.id).await.unwrap(), None);
}

#[tokio::test]
async fn repo_lists_rooms_and_tenants_by_name() {
    let app = test_app().await;
    let db = app.db();
    for name in ["b", "C", "A", "a2"] {
        seed_room(db, name, 1).await;
    }
    let names: Vec<String> = room_repo::get_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(names, ["A", "a2", "b", "C"], "case-insensitive by name");

    let room = seed_room(db, "Room", 1).await;
    for name in ["zed", "Ana", "BEN", "ana b"] {
        seed_tenant(db, room.id, name).await;
    }
    let names: Vec<String> = tenant_repo::get_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, ["Ana", "ana b", "BEN", "zed"]);
}

fn tenant_model(room_id: i32, name: &str) -> tenant::ActiveModel {
    let now = Utc::now().naive_utc();
    tenant::ActiveModel {
        room_id: Set(room_id),
        name: Set(name.to_string()),
        is_active: Set(true),
        join_date: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

#[tokio::test]
async fn repo_create_and_get_tenant() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;

    let created = tenant_repo::create(db, tenant_model(room.id, "John Doe"))
        .await
        .unwrap();
    assert_eq!(created.name, "John Doe");

    let fetched = tenant_repo::get_by_id(db, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.name, created.name);

    let by_name = tenant_repo::get_by_name(db, "John Doe")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_name.id, created.id);
    assert!(
        tenant_repo::get_by_name(db, "john doe")
            .await
            .unwrap()
            .is_none(),
        "exact, case-sensitive match"
    );
}

#[tokio::test]
async fn repo_update_tenant() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;
    let created = tenant_repo::create(db, tenant_model(room.id, "Jane Doe"))
        .await
        .unwrap();

    let mut am: tenant::ActiveModel = created.clone().into();
    am.name = Set("Jane Smith".into());
    let updated = tenant_repo::update(db, created.id, am).await.unwrap();
    assert_eq!(updated.name, "Jane Smith");
}

#[tokio::test]
async fn repo_delete_tenant() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;
    let created = tenant_repo::create(db, tenant_model(room.id, "Delete Me"))
        .await
        .unwrap();

    assert!(tenant_repo::delete(db, created.id).await.unwrap().is_some());
    assert!(
        tenant_repo::get_by_id(db, created.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn repo_create_and_get_reading() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;
    let tenant = seed_tenant(db, room.id, "Test Tenant").await;

    let created = electricity_reading_repo::create(db, reading_model(&tenant, 100, 150))
        .await
        .unwrap();
    let fetched = electricity_reading_repo::get_by_id(db, created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.consumption, 50);
    assert_eq!(fetched.tenant_id, tenant.id);
    assert_eq!(fetched.room_id, room.id);
}

#[tokio::test]
async fn repo_update_reading() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;
    let tenant = seed_tenant(db, room.id, "Test Tenant").await;
    let reading = electricity_reading_repo::create(db, reading_model(&tenant, 100, 150))
        .await
        .unwrap();

    let mut am: electricity_reading::ActiveModel = reading.clone().into();
    am.curr_reading = Set(200);
    am.consumption = Set(200 - reading.prev_reading);
    let result = electricity_reading_repo::update(db, am).await.unwrap();
    assert_eq!(result.curr_reading, 200);
    assert_eq!(result.consumption, 100);
}

#[tokio::test]
async fn repo_delete_reading() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Test Room", 1000).await;
    let tenant = seed_tenant(db, room.id, "Test Tenant").await;
    let reading = electricity_reading_repo::create(db, reading_model(&tenant, 100, 150))
        .await
        .unwrap();

    assert!(
        electricity_reading_repo::delete(db, reading.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        electricity_reading_repo::get_by_id(db, reading.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn repo_lists_readings_newest_first_with_id_tiebreak() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let tenant = seed_tenant(db, room.id, "T").await;
    let at = |day: u32| {
        NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap()
    };
    let mut ids = Vec::new();
    for day in [1, 3, 3, 2] {
        let mut am = reading_model(&tenant, 0, day as i32);
        am.created_at = Set(at(day));
        am.updated_at = Set(at(day));
        ids.push(am.insert(db).await.unwrap().id);
    }
    let order: Vec<i32> = electricity_reading_repo::get_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    // Day 3 (the later id first), day 2, day 1.
    assert_eq!(order, [ids[2], ids[1], ids[3], ids[0]]);
}

// ---- permissions ----

/// Every admin-only route, with a body where it takes one.
fn admin_only_routes(
    room_id: i64,
    tenant_id: i64,
    reading_id: i64,
) -> Vec<(Method, String, Option<Value>)> {
    let room = json!({ "name": "X", "rent": 1 });
    let tenant = json!({ "name": "X", "room_id": room_id, "join_date": JOIN_DATE });
    let reading =
        json!({ "tenant_id": tenant_id, "room_id": room_id, "prev_reading": 1, "curr_reading": 2 });
    vec![
        (Method::GET, "/api/rooms".into(), None),
        (Method::POST, "/api/rooms".into(), Some(room.clone())),
        (Method::GET, format!("/api/rooms/{room_id}"), None),
        (Method::PUT, format!("/api/rooms/{room_id}"), Some(room)),
        (Method::DELETE, format!("/api/rooms/{room_id}"), None),
        (Method::GET, "/api/tenants".into(), None),
        (Method::POST, "/api/tenants".into(), Some(tenant.clone())),
        (Method::GET, "/api/tenants/tenant/ANA".into(), None),
        (
            Method::PUT,
            format!("/api/tenants/{tenant_id}"),
            Some(tenant),
        ),
        (Method::DELETE, format!("/api/tenants/{tenant_id}"), None),
        (Method::GET, "/api/electricity-readings".into(), None),
        (
            Method::POST,
            "/api/electricity-readings".into(),
            Some(reading.clone()),
        ),
        (
            Method::GET,
            format!("/api/electricity-readings/{reading_id}"),
            None,
        ),
        (
            Method::PUT,
            format!("/api/electricity-readings/{reading_id}"),
            Some(reading),
        ),
        (
            Method::DELETE,
            format!("/api/electricity-readings/{reading_id}"),
            None,
        ),
    ]
}

#[tokio::test]
async fn tenants_are_refused_on_admin_routes_and_anonymous_callers_on_all() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let reading = seed_reading(db, &ana, 1, 2).await;
    let tenant_token = app.tenant_token(ana.id, &ana.name);

    for (method, uri, body) in admin_only_routes(room.id.into(), ana.id.into(), reading.id.into()) {
        let (status, reply) = app
            .send(method.clone(), &uri, Some(&tenant_token), body.clone())
            .await;
        assert_eq!(
            (status, reply),
            (StatusCode::FORBIDDEN, error("Admin access required")),
            "{method} {uri} as a tenant"
        );
        let (status, reply) = app.send(method.clone(), &uri, None, body).await;
        assert_eq!(
            (status, reply),
            (StatusCode::UNAUTHORIZED, error("Authentication required")),
            "{method} {uri} without a token"
        );
    }

    // Nothing changed.
    let admin = app.admin_token();
    let (_, rooms) = app.get("/api/rooms", Some(&admin)).await;
    assert_eq!(rooms.as_array().unwrap().len(), 1);
    let (status, _) = app
        .get(
            &format!("/api/electricity-readings/{}", reading.id),
            Some(&admin),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_tenant_reads_only_its_own_tenant_record() {
    let app = test_app().await;
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let ben = seed_tenant(db, room.id, "BEN").await;
    let own = format!("/api/tenants/{}", ana.id);

    let (status, body) = app.get(&own, Some(&app.tenant_token(ana.id, "ANA"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "ANA");

    let (status, admin_view) = app.get(&own, Some(&app.admin_token())).await;
    assert_eq!(
        (status, &admin_view),
        (StatusCode::OK, &body),
        "admin sees the same"
    );

    assert_eq!(
        app.get(&own, Some(&app.tenant_token(ben.id, "BEN"))).await,
        (
            StatusCode::FORBIDDEN,
            error("You can only access your own records")
        )
    );
    assert_eq!(
        app.get(&own, None).await,
        (StatusCode::UNAUTHORIZED, error("Authentication required"))
    );
}

// ---- not found ----

#[tokio::test]
async fn missing_ids_are_404_with_a_message() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let tenant = seed_tenant(db, room.id, "ANA").await;

    let room_body = json!({ "name": "X", "rent": 1 });
    let tenant_body = json!({ "name": "X", "room_id": room.id, "join_date": JOIN_DATE });
    let reading_body =
        json!({ "tenant_id": tenant.id, "room_id": room.id, "prev_reading": 1, "curr_reading": 2 });
    for (what, base, body) in [
        ("Room", "/api/rooms", room_body),
        ("Tenant", "/api/tenants", tenant_body),
        ("Reading", "/api/electricity-readings", reading_body),
    ] {
        let uri = format!("{base}/999");
        let expected = (
            StatusCode::NOT_FOUND,
            error(&format!("{what} 999 not found")),
        );
        assert_eq!(app.get(&uri, token).await, expected, "GET {uri}");
        assert_eq!(app.put(&uri, token, body).await, expected, "PUT {uri}");
        assert_eq!(app.delete(&uri, token).await, expected, "DELETE {uri}");
    }
    assert_eq!(
        app.get("/api/tenants/tenant/Nobody", token).await,
        (StatusCode::NOT_FOUND, error("Tenant \"Nobody\" not found"))
    );
    // A tenant asking for its own (deleted) record.
    assert_eq!(
        app.get("/api/tenants/999", Some(&app.tenant_token(999, "GONE")))
            .await,
        (StatusCode::NOT_FOUND, error("Tenant 999 not found"))
    );
}

// ---- conflicts ----

#[tokio::test]
async fn deleting_referenced_records_is_409() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let reading = seed_reading(db, &ana, 1, 2).await;
    seed_bill(db, &reading, 100, 10).await;

    assert_eq!(
        app.delete(&format!("/api/rooms/{}", room.id), token).await,
        (
            StatusCode::CONFLICT,
            error(&format!("Room {} still has tenants or readings", room.id))
        )
    );
    assert_eq!(
        app.delete(&format!("/api/tenants/{}", ana.id), token).await,
        (
            StatusCode::CONFLICT,
            error(&format!("Tenant {} still has readings or bills", ana.id))
        )
    );
    assert_eq!(
        app.delete(&format!("/api/electricity-readings/{}", reading.id), token)
            .await,
        (
            StatusCode::CONFLICT,
            error(&format!("Reading {} still has a bill", reading.id))
        )
    );
    // Everything is still there.
    for uri in [
        format!("/api/rooms/{}", room.id),
        format!("/api/tenants/{}", ana.id),
        format!("/api/electricity-readings/{}", reading.id),
    ] {
        assert_eq!(app.get(&uri, token).await.0, StatusCode::OK, "{uri}");
    }
}

#[tokio::test]
async fn deleting_a_room_with_only_readings_or_a_tenant_with_only_bills_is_409() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let db = app.db();
    let home = seed_room(db, "Home", 1).await;
    let other = seed_room(db, "Other", 1).await;
    let ana = seed_tenant(db, home.id, "ANA").await;
    let ben = seed_tenant(db, home.id, "BEN").await;
    // A reading of ANA's recorded against the other room, billed to BEN.
    let mut am = reading_model(&ana, 1, 2);
    am.room_id = Set(other.id);
    let reading = am.insert(db).await.unwrap();
    bill::ActiveModel {
        reading_id: Set(reading.id),
        tenant_id: Set(ben.id),
        room_charges: Set(1),
        electric_charges: Set(1),
        total_amount: Set(2),
        receipt_url: Set(None),
        paid: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    assert_eq!(
        app.delete(&format!("/api/rooms/{}", other.id), token).await,
        (
            StatusCode::CONFLICT,
            error(&format!("Room {} still has tenants or readings", other.id))
        )
    );
    assert_eq!(
        app.delete(&format!("/api/tenants/{}", ben.id), token).await,
        (
            StatusCode::CONFLICT,
            error(&format!("Tenant {} still has readings or bills", ben.id))
        )
    );
}

#[tokio::test]
async fn duplicate_names_are_409() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let db = app.db();
    let a = seed_room(db, "101", 1).await;
    let b = seed_room(db, "102", 1).await;
    let ana = seed_tenant(db, a.id, "ANA").await;
    let ben = seed_tenant(db, a.id, "BEN").await;

    let room_taken = (
        StatusCode::CONFLICT,
        error("A room named \"101\" already exists"),
    );
    assert_eq!(
        app.post("/api/rooms", token, json!({ "name": "101", "rent": 1 }))
            .await,
        room_taken
    );
    assert_eq!(
        app.put(
            &format!("/api/rooms/{}", b.id),
            token,
            json!({ "name": "101", "rent": 1 })
        )
        .await,
        room_taken
    );
    // Keeping its own name is fine.
    assert_eq!(
        app.put(
            &format!("/api/rooms/{}", a.id),
            token,
            json!({ "name": "101", "rent": 2 })
        )
        .await
        .0,
        StatusCode::OK
    );

    let tenant_taken = (
        StatusCode::CONFLICT,
        error("A tenant named \"ANA\" already exists"),
    );
    let body = json!({ "name": "ANA", "room_id": a.id, "join_date": JOIN_DATE });
    assert_eq!(
        app.post("/api/tenants", token, body.clone()).await,
        tenant_taken
    );
    assert_eq!(
        app.put(&format!("/api/tenants/{}", ben.id), token, body.clone())
            .await,
        tenant_taken
    );
    assert_eq!(
        app.put(&format!("/api/tenants/{}", ana.id), token, body)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn references_to_missing_rooms_or_tenants_are_409() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let reading = seed_reading(db, &ana, 1, 2).await;

    let no_room = (StatusCode::CONFLICT, error("Room 999 does not exist"));
    let body = json!({ "name": "NEW", "room_id": 999, "join_date": JOIN_DATE });
    assert_eq!(app.post("/api/tenants", token, body.clone()).await, no_room);
    assert_eq!(
        app.put(&format!("/api/tenants/{}", ana.id), token, body)
            .await,
        no_room
    );

    let reading_uri = format!("/api/electricity-readings/{}", reading.id);
    let no_tenant = (StatusCode::CONFLICT, error("Tenant 998 does not exist"));
    let body =
        json!({ "tenant_id": 998, "room_id": room.id, "prev_reading": 1, "curr_reading": 2 });
    assert_eq!(
        app.post("/api/electricity-readings", token, body.clone())
            .await,
        no_tenant
    );
    assert_eq!(app.put(&reading_uri, token, body).await, no_tenant);

    let body = json!({ "tenant_id": ana.id, "room_id": 999, "prev_reading": 1, "curr_reading": 2 });
    assert_eq!(
        app.post("/api/electricity-readings", token, body.clone())
            .await,
        no_room
    );
    assert_eq!(app.put(&reading_uri, token, body).await, no_room);

    // The tenant is reported first when both are missing.
    let body = json!({ "tenant_id": 998, "room_id": 999, "prev_reading": 1, "curr_reading": 2 });
    assert_eq!(
        app.post("/api/electricity-readings", token, body).await,
        no_tenant
    );
}

// ---- validation ----

#[tokio::test]
async fn invalid_path_ids_are_400_naming_the_parameter() {
    let app = test_app().await;
    let token = app.admin_token();
    for uri in [
        "/api/rooms/abc",
        "/api/tenants/tenant",
        "/api/electricity-readings/99999999999",
    ] {
        let (status, body) = app.get(uri, Some(&token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        assert!(error_text(&body).contains("`id`"), "{uri}: {body}");
    }
    let (status, body) = app.delete("/api/rooms/x", Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn invalid_bodies_are_400_naming_the_field() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let room = seed_room(app.db(), "Room", 1).await;

    let cases = [
        ("/api/rooms", json!({ "name": "A" }), "rent"),
        ("/api/rooms", json!({ "name": "A", "rent": "5000" }), "rent"),
        ("/api/rooms", json!({ "name": "A", "rent": 5000.5 }), "rent"),
        ("/api/rooms", json!({ "name": null, "rent": 1 }), "name"),
        (
            "/api/rooms",
            json!({ "name": "A", "rent": 3_000_000_000_i64 }),
            "rent",
        ),
        ("/api/rooms", json!({ "name": "  ", "rent": 1 }), "name"),
        (
            "/api/tenants",
            json!({ "name": "A", "room_id": room.id }),
            "join_date",
        ),
        (
            "/api/tenants",
            json!({ "name": "A", "room_id": room.id, "join_date": "2026-01-01" }),
            "join_date",
        ),
        (
            "/api/tenants",
            json!({ "name": "A", "room_id": room.id, "join_date": "2026-01-01T00:00:00Z" }),
            "join_date",
        ),
        (
            "/api/tenants",
            json!({ "name": "A", "room_id": room.id, "join_date": JOIN_DATE, "is_active": "yes" }),
            "is_active",
        ),
        (
            "/api/tenants",
            json!({ "name": "", "room_id": room.id, "join_date": JOIN_DATE }),
            "name",
        ),
        (
            "/api/electricity-readings",
            json!({ "tenant_id": 1, "room_id": 1, "prev_reading": 1 }),
            "curr_reading",
        ),
        (
            "/api/electricity-readings",
            json!({ "tenant_id": 1, "room_id": 1, "prev_reading": -2_147_483_648_i64, "curr_reading": 2_147_483_647 }),
            "curr_reading - prev_reading",
        ),
    ];
    for (uri, body, field) in cases {
        let (status, reply) = app.post(uri, token, body.clone()).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "POST {uri} {body}: {reply}"
        );
        assert!(
            error_text(&reply).contains(field),
            "POST {uri} {body}: {reply}"
        );
    }
    let (status, reply) = app
        .put(
            &format!("/api/rooms/{}", room.id),
            token,
            json!({ "rent": 1 }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(error_text(&reply).contains("name"), "{reply}");
    assert_eq!(
        app.get("/api/rooms", token).await.1,
        json!([room]),
        "nothing was written"
    );
}

#[tokio::test]
async fn malformed_json_wrong_content_type_and_oversized_bodies_are_json_errors() {
    let app = test_app().await;
    let token = app.admin_token();

    let reply = app
        .request(
            Method::POST,
            "/api/rooms",
            Some(&token),
            Some(("application/json", b"{\"name\": ".to_vec())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(error_text(&reply.json()).contains("JSON"));

    let reply = app
        .request(
            Method::POST,
            "/api/rooms",
            Some(&token),
            Some(("text/plain", br#"{"name":"A","rent":1}"#.to_vec())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(error_text(&reply.json()).contains("Content-Type"));

    let reply = app
        .request(Method::POST, "/api/rooms", Some(&token), None)
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST, "no body at all");
    assert!(reply.json()["error"].is_string());

    let huge = format!(r#"{{"name":"{}","rent":1}}"#, "x".repeat(3 * 1024 * 1024));
    let reply = app
        .request(
            Method::POST,
            "/api/rooms",
            Some(&token),
            Some(("application/json", huge.into_bytes())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(reply.json()["error"].is_string());
}

// ---- other parity details ----

#[tokio::test]
async fn unknown_fields_are_ignored_and_is_active_defaults_to_true_on_update() {
    let app = test_app().await;
    let token = app.admin_token();
    let token = Some(token.as_str());
    let room = seed_room(app.db(), "Room", 1).await;

    let (status, tenant) = app
        .post(
            "/api/tenants",
            token,
            json!({ "id": 77, "name": "ANA", "room_id": room.id, "join_date": "2026-03-04T05:06:07.5", "is_active": false, "created_at": "x" }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{tenant}");
    assert_ne!(tenant["id"], 77);
    assert_eq!(tenant["is_active"], false);
    assert_eq!(tenant["join_date"], "2026-03-04T05:06:07.500");

    let (status, updated) = app
        .put(
            &format!("/api/tenants/{}", id_of(&tenant)),
            token,
            json!({ "name": "ANA", "room_id": room.id, "join_date": JOIN_DATE, "is_active": null }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        updated["is_active"], true,
        "null/omitted re-activates, as before"
    );
}

#[tokio::test]
async fn negative_consumption_is_accepted() {
    let app = test_app().await;
    let token = app.admin_token();
    let db = app.db();
    let room = seed_room(db, "Room", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let (status, reading) = app
        .post(
            "/api/electricity-readings",
            Some(&token),
            json!({ "tenant_id": ana.id, "room_id": room.id, "prev_reading": 150, "curr_reading": 100 }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(reading["consumption"], -50);
}

#[tokio::test]
async fn lists_are_sorted_through_the_api() {
    let app = test_app().await;
    let token = app.admin_token();
    let db = app.db();
    for name in ["b", "A", "C"] {
        seed_room(db, name, 1).await;
    }
    let (_, rooms) = app.get("/api/rooms", Some(&token)).await;
    let names: Vec<&str> = rooms
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["A", "b", "C"]);

    // Same-second readings: the newer id first.
    let room = seed_room(db, "R", 1).await;
    let ana = seed_tenant(db, room.id, "ANA").await;
    let first = seed_reading(db, &ana, 0, 1).await;
    let second = seed_reading(db, &ana, 1, 2).await;
    db.execute_unprepared("UPDATE electricity_reading SET created_at = '2026-09-01 08:00:00'")
        .await
        .unwrap();
    let (_, readings) = app.get("/api/electricity-readings", Some(&token)).await;
    let ids: Vec<i64> = readings.as_array().unwrap().iter().map(id_of).collect();
    assert_eq!(ids, [i64::from(second.id), i64::from(first.id)]);
}

#[tokio::test]
async fn wrong_methods_are_405() {
    let app = test_app().await;
    let token = app.admin_token();
    let (status, _) = app
        .send(
            Method::DELETE,
            "/api/tenants/tenant/ANA",
            Some(&token),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    let (status, _) = app
        .send(Method::PATCH, "/api/rooms/1", Some(&token), None)
        .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

// ---- error mapping ----

#[test]
fn classifies_constraint_failures() {
    assert_eq!(
        violation(&DbErr::Custom("UNIQUE constraint failed: room.name".into())),
        Some(Violation::Unique)
    );
    assert_eq!(
        violation(&DbErr::Custom(
            "D1_ERROR: FOREIGN KEY constraint failed: SQLITE_CONSTRAINT".into()
        )),
        Some(Violation::ForeignKey)
    );
    assert_eq!(violation(&DbErr::Custom("disk on fire".into())), None);
    assert_eq!(violation(&DbErr::RecordNotUpdated), None);
}

#[test]
fn consumption_is_curr_minus_prev_and_refuses_overflow() {
    assert_eq!(calculate_consumption(100, 150), Ok(50));
    assert_eq!(calculate_consumption(150, 100), Ok(-50));
    let err = calculate_consumption(i32::MIN, i32::MAX).unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn database_failures_are_500_without_details() {
    let app = test_app().await;
    let token = app.admin_token();
    app.db()
        .execute_unprepared(
            "DROP TABLE additional_charge; DROP TABLE bill; DROP TABLE electricity_reading",
        )
        .await
        .unwrap();
    assert_eq!(
        app.get("/api/electricity-readings", Some(&token)).await,
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            error("Internal server error")
        )
    );
}
