//! Flows across the domains, through the real routes only (accounts → property
//! → billing), and the export of the contract fixtures the Flutter apps test against.
mod common;

use std::path::PathBuf;

use axum::http::{Method, StatusCode, header};
use common::{ADMIN_PASSWORD, ADMIN_USERNAME, Multipart, TestApp, samples, test_app};
use serde_json::{Value, json};

// Synthetic data (also used for the exported fixtures).
const ROOM_NAME: &str = "Room 101";
const ROOM_RENT: i32 = 5000;
const TENANT_NAME: &str = "Juan Dela Cruz";
const JOIN_DATE: &str = "2026-01-01T00:00:00";

// JSON keys the Flutter apps read.
const ROOM_KEYS: &[&str] = &["id", "name", "rent"];
const TENANT_KEYS: &[&str] = &["id", "room_id", "name", "join_date", "is_active"];
const READING_KEYS: &[&str] = &[
    "id",
    "tenant_id",
    "room_id",
    "prev_reading",
    "curr_reading",
    "consumption",
    "created_at",
];
const BILL_KEYS: &[&str] = &[
    "id",
    "reading_id",
    "tenant_id",
    "room_charges",
    "electric_charges",
    "total_amount",
    "created_at",
    "paid",
    "receipt_url",
];
const CHARGE_KEYS: &[&str] = &["amount", "description"];

fn assert_keys(value: &Value, keys: &[&str], what: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what}: expected an object, got {value}"));
    for key in keys {
        assert!(
            object.contains_key(*key),
            "{what}: missing `{key}` in {value}"
        );
    }
}

fn assert_bill_details(value: &Value, what: &str) {
    assert_keys(value, &["bill", "additional_charges", "reading"], what);
    assert_keys(&value["bill"], BILL_KEYS, &format!("{what}.bill"));
    for charge in value["additional_charges"]
        .as_array()
        .expect("charges array")
    {
        assert_keys(charge, CHARGE_KEYS, &format!("{what}.additional_charges[]"));
    }
    assert_keys(&value["reading"], READING_KEYS, &format!("{what}.reading"));
}

fn token_of(body: &Value) -> String {
    body["token"]
        .as_str()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| panic!("no token in {body}"))
        .to_string()
}

/// The path and query of an absolute URL from this API (`http://api.test/...`).
fn path_of(url: &str) -> &str {
    url.strip_prefix("http://api.test")
        .unwrap_or_else(|| panic!("unexpected link {url}"))
}

/// Everything the admin flow produced, as the API returned it.
struct Flow {
    admin: Value,
    room: Value,
    tenant: Value,
    reading: Value,
    created_bill: Value,
    bills: Value,
    latest_bill: Value,
    tenant_bills: Value,
}

/// Admin login → room → tenant → reading → bill → the three bill reads, all through the routes.
async fn admin_flow(app: &TestApp) -> Flow {
    let (status, admin) = app
        .post(
            "/api/auth/admin-login",
            None,
            json!({ "username": ADMIN_USERNAME, "password": ADMIN_PASSWORD }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "admin login: {admin}");
    let token = token_of(&admin);
    let token = Some(token.as_str());

    let (status, room) = app
        .post(
            "/api/rooms",
            token,
            json!({ "name": ROOM_NAME, "rent": ROOM_RENT }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "room: {room}");
    assert_keys(&room, ROOM_KEYS, "room");

    let (status, tenant) = app
        .post(
            "/api/tenants",
            token,
            json!({ "name": TENANT_NAME, "room_id": room["id"], "join_date": JOIN_DATE }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "tenant: {tenant}");
    assert_keys(&tenant, TENANT_KEYS, "tenant");

    let (status, reading) = app
        .post("/api/electricity-readings", token, json!({ "tenant_id": tenant["id"], "room_id": room["id"], "prev_reading": 100, "curr_reading": 150 }))
        .await;
    assert_eq!(status, StatusCode::CREATED, "reading: {reading}");
    assert_keys(&reading, READING_KEYS, "reading");
    assert_eq!(reading["consumption"], 50);

    let (status, created_bill) = app
        .post(
            "/api/bills",
            token,
            json!({
                "tenant_id": tenant["id"],
                "reading_id": reading["id"],
                "room_charges": 5000,
                "electric_charges": 850,
                "additional_charges": [{ "amount": 200, "description": "Water" }],
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "bill: {created_bill}");
    assert_bill_details(&created_bill, "created bill");
    assert_eq!(created_bill["bill"]["total_amount"], 6050);
    assert_eq!(created_bill["reading"], reading);

    let tenant_id = tenant["id"].as_i64().unwrap();
    let (status, bills) = app.get("/api/bills", token).await;
    assert_eq!(status, StatusCode::OK);
    let (status, latest_bill) = app
        .get(&format!("/api/bills/{tenant_id}/bill"), token)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, tenant_bills) = app
        .get(&format!("/api/bills/{tenant_id}/bills"), token)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bills, json!([created_bill.clone()]));
    assert_eq!(latest_bill, created_bill);
    assert_eq!(tenant_bills, json!([created_bill.clone()]));

    Flow {
        admin,
        room,
        tenant,
        reading,
        created_bill,
        bills,
        latest_bill,
        tenant_bills,
    }
}

#[tokio::test]
async fn admin_bills_a_tenant_who_then_sees_the_bill_and_its_receipt() {
    let app = test_app().await;
    let flow = admin_flow(&app).await;
    let admin = token_of(&flow.admin);
    let tenant_id = flow.tenant["id"].as_i64().unwrap();
    let bill_id = flow.created_bill["bill"]["id"].as_i64().unwrap();

    // The tenant logs in by name and reads only its own records.
    let (status, login) = app
        .post("/api/auth/login", None, json!({ "name": TENANT_NAME }))
        .await;
    assert_eq!(status, StatusCode::OK, "tenant login: {login}");
    assert_eq!(login["tenant"], flow.tenant);
    let tenant = token_of(&login);
    let (status, me) = app
        .get(&format!("/api/tenants/{tenant_id}"), Some(&tenant))
        .await;
    assert_eq!((status, me), (StatusCode::OK, flow.tenant.clone()));
    let (status, latest) = app
        .get(&format!("/api/bills/{tenant_id}/bill"), Some(&tenant))
        .await;
    assert_eq!(
        (status, latest),
        (StatusCode::OK, flow.created_bill.clone())
    );
    let (status, _) = app.get("/api/bills", Some(&tenant)).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "the full bill list is admin-only"
    );
    let (status, _) = app
        .post(
            "/api/rooms",
            Some(&tenant),
            json!({ "name": "X", "rent": 1 }),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "tenants can't change property"
    );

    // The admin attaches a receipt: the bill becomes paid.
    let form = Multipart::default()
        .field("tenant_id", tenant_id)
        .field("reading_id", flow.reading["id"].as_i64().unwrap())
        .field("room_charges", 5000)
        .field("electric_charges", 850)
        .field(
            "additional_charges",
            r#"[{"amount":200,"description":"Water"}]"#,
        )
        .file("receipt_file", "receipt.png", "image/png", samples::PNG);
    let (status, paid) = app
        .put_multipart(&format!("/api/bills/{bill_id}/upload"), Some(&admin), form)
        .await;
    assert_eq!(status, StatusCode::OK, "upload: {paid}");
    assert_eq!(paid["bill"]["paid"], true);
    let receipt = paid["bill"]["receipt_url"].as_str().unwrap().to_string();

    // The tenant opens its receipt through a signed link; other tenants can't get one.
    let (status, link) = app
        .get(
            &format!(
                "/api/signed-urls/receipts/{}/{receipt}",
                TENANT_NAME.replace(' ', "%20")
            ),
            Some(&tenant),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "receipt link: {link}");
    assert_eq!(link["content_type"], "image/png");
    let file = app
        .request(
            Method::GET,
            path_of(link["url"].as_str().unwrap()),
            None,
            None,
        )
        .await;
    assert_eq!(file.status, StatusCode::OK);
    assert_eq!(file.headers.get(header::CONTENT_TYPE).unwrap(), "image/png");
    assert_eq!(file.body, samples::PNG);
    let stranger = app.tenant_token(tenant_id as i32 + 1, "SOMEONE ELSE");
    let (status, _) = app
        .get(
            &format!(
                "/api/signed-urls/receipts/{}/{receipt}",
                TENANT_NAME.replace(' ', "%20")
            ),
            Some(&stranger),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Clean-up in dependency order through the routes.
    for path in [
        format!("/api/bills/{bill_id}"),
        format!("/api/electricity-readings/{}", flow.reading["id"]),
        format!("/api/tenants/{tenant_id}"),
        format!("/api/rooms/{}", flow.room["id"]),
    ] {
        let (status, body) = app.delete(&path, Some(&admin)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{path}: {body}");
    }
    let (_, rooms) = app.get("/api/rooms", Some(&admin)).await;
    assert_eq!(rooms, json!([]));
}

#[tokio::test]
async fn a_room_in_use_cannot_be_deleted_until_its_records_are_gone() {
    let app = test_app().await;
    let flow = admin_flow(&app).await;
    let admin = token_of(&flow.admin);
    let room = format!("/api/rooms/{}", flow.room["id"]);

    let (status, body) = app.delete(&room, Some(&admin)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("still has"),
        "{body}"
    );
    let (status, rooms) = app.get("/api/rooms", Some(&admin)).await;
    assert_eq!((status, rooms), (StatusCode::OK, json!([flow.room])));
}

fn is_jwt(s: &str) -> bool {
    let segments: Vec<&str> = s.split('.').collect();
    s.starts_with("eyJ")
        && segments.len() == 3
        && segments.iter().all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
}

fn redact(value: &mut Value) {
    match value {
        Value::String(s) if is_jwt(s) => *s = "<token>".into(),
        Value::String(s) if s.contains("/api/files/") && s.contains("signature=") => {
            *s = "<signed-url>".into()
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::Object(map) => map.values_mut().for_each(redact),
        _ => {}
    }
}

/// Writes the API responses the Flutter apps consume as JSON fixtures:
/// `$env:FIXTURES_OUT='C:\dev\shared-packages\packages\m18_residences_shared\test\fixtures'; cargo test export_contract_fixtures -- --ignored`
#[tokio::test]
#[ignore = "writes contract fixtures; needs FIXTURES_OUT and --ignored"]
async fn export_contract_fixtures() {
    let out_dir = std::env::var_os("FIXTURES_OUT")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .expect("set FIXTURES_OUT to the fixtures directory (see the doc comment)");

    let app = test_app().await;
    let flow = admin_flow(&app).await;
    let (status, tenant_login) = app
        .post("/api/auth/login", None, json!({ "name": TENANT_NAME }))
        .await;
    assert_eq!(status, StatusCode::OK);
    app.files
        .insert("payments/gcash.png", samples::PNG, "image/png");
    let (status, signed_url) = app
        .get(
            "/api/signed-urls/payments/gcash",
            Some(&token_of(&flow.admin)),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{signed_url}");
    let _ = flow.tenant_bills;

    std::fs::create_dir_all(&out_dir).unwrap();
    let fixtures = [
        ("admin_login.json", flow.admin),
        ("tenant_login.json", tenant_login),
        ("room.json", flow.room),
        ("tenant.json", flow.tenant),
        ("reading.json", flow.reading),
        ("bill.json", flow.created_bill),
        ("bills.json", flow.bills),
        ("latest_bill.json", flow.latest_bill),
        ("signed_url.json", signed_url),
    ];
    for (name, mut value) in fixtures {
        redact(&mut value);
        let text = serde_json::to_string_pretty(&value).unwrap();
        assert!(
            !text.contains("eyJ") && !text.contains("signature="),
            "{name}: token or signed link left: {text}"
        );
        let path = out_dir.join(name);
        std::fs::write(&path, text + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}
