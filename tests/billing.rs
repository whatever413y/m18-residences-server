//! Billing: bills, additional charges, receipt and payment uploads, signed links and the
//! file route. Other tables are seeded directly; tokens are minted directly.
mod common;

use axum::http::{Method, StatusCode, header};
use common::{
    Multipart, TestApp, samples, seed_bill, seed_reading, seed_room, seed_tenant, test_app,
};
use m18_residences_db::entities::{bill, electricity_reading, room, tenant};
use m18_residences_server::billing::repository::{
    additional_charge_repo, bill_repo, reading_read_repo, tenant_read_repo,
};
use m18_residences_shared_rs::files::FileSigner;
use serde_json::{Value, json};

const ROOM_NAME: &str = "Room 101";
const TENANT_NAME: &str = "Juan Dela Cruz";

const BILL_KEYS: [&str; 11] = [
    "id",
    "reading_id",
    "tenant_id",
    "room_charges",
    "electric_charges",
    "total_amount",
    "receipt_url",
    "payment_url",
    "paid",
    "created_at",
    "updated_at",
];
const CHARGE_KEYS: [&str; 6] = [
    "id",
    "bill_id",
    "amount",
    "description",
    "created_at",
    "updated_at",
];
const READING_KEYS: [&str; 8] = [
    "id",
    "tenant_id",
    "room_id",
    "prev_reading",
    "curr_reading",
    "consumption",
    "created_at",
    "updated_at",
];

// ---------- helpers ----------

struct World {
    room: room::Model,
    tenant: tenant::Model,
    reading: electricity_reading::Model,
}

/// A room, a tenant and a 100 → 150 reading.
async fn world(app: &TestApp) -> World {
    let room = seed_room(app.db(), ROOM_NAME, 5000).await;
    let tenant = seed_tenant(app.db(), room.id, TENANT_NAME).await;
    let reading = seed_reading(app.db(), &tenant, 100, 150).await;
    World {
        room,
        tenant,
        reading,
    }
}

fn bill_body(w: &World, reading_id: i32) -> Value {
    json!({
        "tenant_id": w.tenant.id,
        "reading_id": reading_id,
        "room_charges": 5000,
        "electric_charges": 850,
        "additional_charges": [{ "amount": 200, "description": "Water" }],
    })
}

async fn create_bill(app: &TestApp, w: &World) -> Value {
    let (status, body) = app
        .post(
            "/api/bills",
            Some(&app.admin_token()),
            bill_body(w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "create bill: {body}");
    body
}

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
    assert_keys(&value["bill"], &BILL_KEYS, &format!("{what}.bill"));
    for charge in value["additional_charges"]
        .as_array()
        .unwrap_or_else(|| panic!("{what}.additional_charges: expected an array in {value}"))
    {
        assert_keys(
            charge,
            &CHARGE_KEYS,
            &format!("{what}.additional_charges[]"),
        );
    }
    assert_keys(&value["reading"], &READING_KEYS, &format!("{what}.reading"));
}

fn error(message: &str) -> Value {
    json!({ "error": message })
}

fn error_text(body: &Value) -> String {
    body["error"]
        .as_str()
        .unwrap_or_else(|| panic!("expected a JSON error, got {body}"))
        .to_string()
}

fn upload_form(w: &World, reading_id: i32) -> Multipart {
    Multipart::default()
        .field("tenant_id", w.tenant.id)
        .field("reading_id", reading_id)
        .field("room_charges", 5000)
        .field("electric_charges", 1190)
        .field(
            "additional_charges",
            json!([{ "amount": 100, "description": "Water" }]),
        )
}

fn path_and_query(url: &str) -> &str {
    url.strip_prefix("http://api.test")
        .unwrap_or_else(|| panic!("expected a link on http://api.test, got {url}"))
}

// ---------- the legacy API flow ----------

#[tokio::test]
async fn admin_billing_flow_returns_contract_shapes() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();

    let created = create_bill(&app, &w).await;
    assert_bill_details(&created, "created bill");
    let bill = &created["bill"];
    assert_eq!(bill["tenant_id"], w.tenant.id);
    assert_eq!(bill["reading_id"], w.reading.id);
    assert_eq!(bill["room_charges"], 5000);
    assert_eq!(bill["electric_charges"], 850);
    assert_eq!(
        bill["total_amount"], 6050,
        "total = room + electric + additional"
    );
    assert_eq!(bill["paid"], false);
    assert_eq!(bill["receipt_url"], Value::Null);
    assert_eq!(
        created["reading"],
        serde_json::to_value(&w.reading).unwrap()
    );
    let charges = created["additional_charges"].as_array().unwrap();
    assert_eq!(charges.len(), 1);
    assert_eq!(charges[0]["amount"], 200);
    assert_eq!(charges[0]["description"], "Water");
    assert_eq!(charges[0]["bill_id"], bill["id"]);

    let (status, bills) = app.get("/api/bills", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bills, json!([created.clone()]));

    let (status, latest) = app
        .get(&format!("/api/bills/{}/bill", w.tenant.id), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(latest, created);

    let (status, tenant_bills) = app
        .get(&format!("/api/bills/{}/bills", w.tenant.id), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tenant_bills, json!([created]));
}

#[tokio::test]
async fn tenant_reads_own_bills_only() {
    let app = test_app().await;
    let w = world(&app).await;
    let other = seed_tenant(app.db(), w.room.id, "Other Tenant").await;
    let created = create_bill(&app, &w).await;
    let own = app.tenant_token(w.tenant.id, &w.tenant.name);
    let foreign = app.tenant_token(other.id, &other.name);

    for path in ["bill", "bills"] {
        let uri = format!("/api/bills/{}/{path}", w.tenant.id);
        let (status, body) = app.get(&uri, Some(&own)).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        let expected = if path == "bill" {
            created.clone()
        } else {
            json!([created.clone()])
        };
        assert_eq!(body, expected, "{uri}");

        let (status, body) = app.get(&uri, Some(&foreign)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(body, error("You can only access your own records"));

        let (status, body) = app.get(&uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(body, error("Authentication required"));

        let (status, _) = app.get(&uri, Some("not-a-jwt")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }

    // The other tenant has no bills: 404 for the latest one, [] for the list.
    let (status, body) = app
        .get(&format!("/api/bills/{}/bill", other.id), Some(&foreign))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        error(&format!("No bill found for tenant {}", other.id))
    );
    let (status, body) = app
        .get(&format!("/api/bills/{}/bills", other.id), Some(&foreign))
        .await;
    assert_eq!((status, body), (StatusCode::OK, json!([])));
}

#[tokio::test]
async fn latest_bill_is_404_with_a_json_error_for_tenant_without_bills() {
    let app = test_app().await;
    let room = seed_room(app.db(), ROOM_NAME, 5000).await;
    let tenant = seed_tenant(app.db(), room.id, TENANT_NAME).await;
    let (status, body) = app
        .get(
            &format!("/api/bills/{}/bill", tenant.id),
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        error(&format!("No bill found for tenant {}", tenant.id))
    );
}

#[tokio::test]
async fn bills_update_through_json_and_multipart_then_delete() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].as_i64().unwrap();
    let old_charge_id = created["additional_charges"][0]["id"].clone();

    let (status, updated) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&token),
            json!({
                "tenant_id": w.tenant.id,
                "reading_id": w.reading.id,
                "room_charges": 5000,
                "electric_charges": 1190,
                "additional_charges": [{ "amount": 300, "description": "Internet" }],
                "receipt_url": null,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_bill_details(&updated, "updated bill");
    assert_eq!(updated["bill"]["total_amount"], 6490);
    assert_eq!(updated["bill"]["paid"], false);
    assert_eq!(updated["bill"]["created_at"], created["bill"]["created_at"]);
    assert_eq!(updated["bill"]["updated_at"], created["bill"]["updated_at"]);
    let charges = updated["additional_charges"].as_array().unwrap();
    assert_eq!(charges.len(), 1);
    assert_eq!(charges[0]["description"], "Internet");
    assert_ne!(charges[0]["id"], old_charge_id, "charges are replaced");

    let (status, uploaded) = app
        .put_multipart(
            &format!("/api/bills/{bill_id}/upload"),
            Some(&token),
            upload_form(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{uploaded}");
    assert_bill_details(&uploaded, "multipart update");
    assert_eq!(uploaded["bill"]["total_amount"], 6290);
    assert_eq!(uploaded["bill"]["paid"], false);
    assert_eq!(uploaded["bill"]["receipt_url"], Value::Null);
    assert!(app.files.keys().is_empty(), "no file part, nothing stored");

    let (status, body) = app
        .delete(&format!("/api/bills/{bill_id}"), Some(&token))
        .await;
    assert_eq!((status, body), (StatusCode::NO_CONTENT, Value::Null));
    assert!(
        additional_charge_repo::get_all_by_bill_id(app.db(), bill_id as i32)
            .await
            .unwrap()
            .is_empty()
    );
    let (status, body) = app.get("/api/bills", Some(&token)).await;
    assert_eq!((status, body), (StatusCode::OK, json!([])));

    // Deleting again, or updating a deleted bill: 404 (was a bare 404 / 500).
    let (status, body) = app
        .delete(&format!("/api/bills/{bill_id}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error(&format!("Bill {bill_id} not found")));
    let (status, body) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&token),
            bill_body(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error(&format!("Bill {bill_id} not found")));
    let (status, body) = app
        .put_multipart(
            &format!("/api/bills/{bill_id}/upload"),
            Some(&token),
            upload_form(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error(&format!("Bill {bill_id} not found")));
}

#[tokio::test]
async fn receipt_url_sets_paid_and_an_empty_one_means_no_receipt() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let uri = format!("/api/bills/{}", created["bill"]["id"]);

    let mut body = bill_body(&w, w.reading.id);
    body["receipt_url"] = json!("1700000000-r1");
    let (status, updated) = app.put(&uri, Some(&token), body.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["bill"]["receipt_url"], "1700000000-r1");
    assert_eq!(updated["bill"]["paid"], true);

    body["receipt_url"] = json!("");
    let (_, updated) = app.put(&uri, Some(&token), body.clone()).await;
    assert_eq!(updated["bill"]["receipt_url"], Value::Null);
    assert_eq!(updated["bill"]["paid"], false);

    // Omitted: no receipt (as before).
    body["receipt_url"] = json!("x");
    app.put(&uri, Some(&token), body.clone()).await;
    body.as_object_mut().unwrap().remove("receipt_url");
    let (_, updated) = app.put(&uri, Some(&token), body).await;
    assert_eq!(updated["bill"]["paid"], false);

    // Multipart without a file: the receipt_url field behaves the same.
    let upload = format!("{uri}/upload");
    let (status, updated) = app
        .put_multipart(
            &upload,
            Some(&token),
            upload_form(&w, w.reading.id).field("receipt_url", "1700000000-r1"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["bill"]["paid"], true);
    assert_eq!(updated["bill"]["receipt_url"], "1700000000-r1");
    let (_, updated) = app
        .put_multipart(
            &upload,
            Some(&token),
            upload_form(&w, w.reading.id).field("receipt_url", ""),
        )
        .await;
    assert_eq!(updated["bill"]["paid"], false);
    assert_eq!(updated["bill"]["receipt_url"], Value::Null);

    // A new bill never starts with a receipt.
    let reading = seed_reading(app.db(), &w.tenant, 150, 180).await;
    let mut body = bill_body(&w, reading.id);
    body["receipt_url"] = json!("ignored");
    let (status, created) = app.post("/api/bills", Some(&token), body).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["bill"]["paid"], false);
    assert_eq!(created["bill"]["receipt_url"], Value::Null);
}

#[tokio::test]
async fn bills_are_newest_first_and_charges_oldest_first() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let second_reading = seed_reading(app.db(), &w.tenant, 150, 200).await;

    let first = create_bill(&app, &w).await;
    let mut body = bill_body(&w, second_reading.id);
    body["additional_charges"] = json!([
        { "amount": 1, "description": "A" },
        { "amount": 2, "description": "B" },
        { "amount": 3, "description": "C" },
    ]);
    let (status, second) = app.post("/api/bills", Some(&token), body).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(second["bill"]["total_amount"], 5000 + 850 + 6);
    let descriptions: Vec<&str> = second["additional_charges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["description"].as_str().unwrap())
        .collect();
    assert_eq!(descriptions, ["A", "B", "C"]);

    // Both were created within the same second or so: ties go to the higher id.
    let (_, bills) = app.get("/api/bills", Some(&token)).await;
    assert_eq!(bills, json!([second.clone(), first.clone()]));
    let (_, latest) = app
        .get(&format!("/api/bills/{}/bill", w.tenant.id), Some(&token))
        .await;
    assert_eq!(latest, second);
    let (_, tenant_bills) = app
        .get(&format!("/api/bills/{}/bills", w.tenant.id), Some(&token))
        .await;
    assert_eq!(tenant_bills, json!([second, first]));
}

#[tokio::test]
async fn lists_assemble_many_bills_across_query_chunks() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    // More bills than one `IN (...)` query takes on D1.
    for i in 0..105 {
        let reading = seed_reading(app.db(), &w.tenant, i, i + 10).await;
        let mut body = bill_body(&w, reading.id);
        body["additional_charges"] = json!([{ "amount": i, "description": format!("charge {i}") }]);
        let (status, _) = app.post("/api/bills", Some(&token), body).await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, bills) = app.get("/api/bills", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let bills = bills.as_array().unwrap();
    assert_eq!(bills.len(), 105);
    for details in bills {
        assert_bill_details(details, "bills[]");
        assert_eq!(details["reading"]["id"], details["bill"]["reading_id"]);
        let charges = details["additional_charges"].as_array().unwrap();
        assert_eq!(charges.len(), 1);
        assert_eq!(charges[0]["bill_id"], details["bill"]["id"]);
        assert_eq!(
            charges[0]["amount"], details["reading"]["prev_reading"],
            "each bill gets its own charge"
        );
    }
}

// ---------- validation and conflicts ----------

#[tokio::test]
async fn create_and_update_reject_invalid_json_with_the_field_named() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let put_uri = format!("/api/bills/{}", created["bill"]["id"]);

    for (method, uri) in [
        (Method::POST, "/api/bills"),
        (Method::PUT, put_uri.as_str()),
    ] {
        let mut body = bill_body(&w, w.reading.id);
        body.as_object_mut().unwrap().remove("tenant_id");
        let (status, reply) = app
            .send(method.clone(), uri, Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}");
        assert!(error_text(&reply).contains("tenant_id"), "{reply}");

        let mut body = bill_body(&w, w.reading.id);
        body["room_charges"] = json!("5000");
        let (status, reply) = app
            .send(method.clone(), uri, Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error_text(&reply).contains("room_charges"), "{reply}");

        let mut body = bill_body(&w, w.reading.id);
        body["additional_charges"] = json!([{ "amount": 1 }]);
        let (status, reply) = app
            .send(method.clone(), uri, Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(error_text(&reply).contains("description"), "{reply}");

        let mut body = bill_body(&w, w.reading.id);
        body["electric_charges"] = json!(i32::MAX);
        let (status, reply) = app
            .send(method.clone(), uri, Some(&token), Some(body))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(reply, error("The total amount is too large"));

        // Not JSON at all, and no JSON content type: JSON 400s too.
        let reply = app
            .request(
                method.clone(),
                uri,
                Some(&token),
                Some(("application/json", b"{not json".to_vec())),
            )
            .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        assert!(reply.json()["error"].is_string());
        let reply = app
            .request(
                method.clone(),
                uri,
                Some(&token),
                Some(("text/plain", b"{}".to_vec())),
            )
            .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        assert!(reply.json()["error"].is_string());
    }

    // A path id that is not an integer.
    for (method, uri) in [
        (Method::PUT, "/api/bills/abc"),
        (Method::DELETE, "/api/bills/abc"),
        (Method::GET, "/api/bills/abc/bill"),
        (Method::GET, "/api/bills/abc/bills"),
    ] {
        let body = (method == Method::PUT).then(|| bill_body(&w, w.reading.id));
        let (status, reply) = app.send(method.clone(), uri, Some(&token), body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {uri}");
        assert!(error_text(&reply).contains("abc"), "{reply}");
    }
}

#[tokio::test]
async fn conflicts_get_specific_messages() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;

    // A second bill for the same reading.
    let (status, body) = app
        .post("/api/bills", Some(&token), bill_body(&w, w.reading.id))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        error(&format!("Reading {} already has a bill", w.reading.id))
    );

    // Unknown tenant or reading.
    let mut body = bill_body(&w, w.reading.id);
    body["tenant_id"] = json!(999);
    let (status, reply) = app.post("/api/bills", Some(&token), body).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(reply, error("Tenant 999 does not exist"));
    let (status, reply) = app
        .post("/api/bills", Some(&token), bill_body(&w, 999))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(reply, error("Reading 999 does not exist"));

    // Moving a bill onto a reading another bill already has.
    let reading = seed_reading(app.db(), &w.tenant, 150, 200).await;
    let (status, other) = app
        .post("/api/bills", Some(&token), bill_body(&w, reading.id))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, reply) = app
        .put(
            &format!("/api/bills/{}", other["bill"]["id"]),
            Some(&token),
            bill_body(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        reply,
        error(&format!("Reading {} already has a bill", w.reading.id))
    );
    // Updating a bill with its own reading is fine.
    let (status, _) = app
        .put(
            &format!("/api/bills/{}", created["bill"]["id"]),
            Some(&token),
            bill_body(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Nothing was written by the refused requests.
    let (_, bills) = app.get("/api/bills", Some(&token)).await;
    assert_eq!(bills.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn constraint_failures_map_to_409_and_roll_back() {
    // Statements that pass the pre-checks but break a constraint (e.g. a race)
    // leave nothing behind and surface as 409.
    let app = test_app().await;
    let w = world(&app).await;
    let db = &app.state.db;
    let backend = db.backend();
    let first = seed_bill(app.db(), &w.reading, 100, 50).await;
    let am = bill::ActiveModel {
        reading_id: sea_orm::Set(w.reading.id),
        tenant_id: sea_orm::Set(w.tenant.id),
        room_charges: sea_orm::Set(1),
        electric_charges: sea_orm::Set(1),
        total_amount: sea_orm::Set(2),
        paid: sea_orm::Set(false),
        receipt_url: sea_orm::Set(None),
        ..Default::default()
    };
    let err = db
        .atomic(vec![
            additional_charge_repo::insert_statement(backend, first.id, 5, "kept?"),
            bill_repo::insert_statement(backend, am),
        ])
        .await
        .expect_err("duplicate reading");
    let api = m18_residences_shared_rs::error::ApiError::from(err);
    assert_eq!(api.status(), StatusCode::CONFLICT);
    assert!(
        additional_charge_repo::get_all_by_bill_id(app.db(), first.id)
            .await
            .unwrap()
            .is_empty(),
        "the batch was rolled back"
    );
}

#[tokio::test]
async fn admin_only_routes_refuse_tenants_and_anonymous_callers() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let tenant = app.tenant_token(w.tenant.id, &w.tenant.name);

    let requests: Vec<(Method, String, Option<Value>)> = vec![
        (Method::GET, "/api/bills".into(), None),
        (
            Method::POST,
            "/api/bills".into(),
            Some(bill_body(&w, w.reading.id)),
        ),
        (
            Method::PUT,
            format!("/api/bills/{bill_id}"),
            Some(bill_body(&w, w.reading.id)),
        ),
        (Method::DELETE, format!("/api/bills/{bill_id}"), None),
    ];
    for (method, uri, body) in requests {
        let (status, reply) = app
            .send(method.clone(), &uri, Some(&tenant), body.clone())
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert_eq!(reply, error("Admin access required"));
        let (status, reply) = app.send(method.clone(), &uri, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(reply, error("Authentication required"));
    }

    let upload = format!("/api/bills/{bill_id}/upload");
    let form =
        || upload_form(&w, w.reading.id).file("receipt_file", "r.jpg", "image/jpeg", samples::JPEG);
    let (status, reply) = app.put_multipart(&upload, Some(&tenant), form()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(reply, error("Admin access required"));
    let (status, _) = app.put_multipart(&upload, None, form()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(app.files.keys().is_empty());

    // The bill is untouched.
    let (_, latest) = app
        .get(
            &format!("/api/bills/{}/bill", w.tenant.id),
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(latest, created);
}

// ---------- receipt uploads ----------

#[tokio::test]
async fn upload_stores_the_receipt_by_its_sniffed_type() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let upload = format!("/api/bills/{}/upload", created["bill"]["id"]);

    let cases: [(&[u8], &str); 6] = [
        (samples::JPEG, "image/jpeg"),
        (samples::PNG, "image/png"),
        (samples::GIF, "image/gif"),
        (samples::WEBP, "image/webp"),
        (samples::AVIF, "image/avif"),
        (samples::PDF, "application/pdf"),
    ];
    for (bytes, expected_type) in cases {
        let before = chrono::Utc::now().timestamp();
        // The declared type is ignored.
        let form =
            upload_form(&w, w.reading.id).file("receipt_file", "receipt.bin", "image/png", bytes);
        let (status, updated) = app.put_multipart(&upload, Some(&token), form).await;
        assert_eq!(status, StatusCode::OK, "{expected_type}: {updated}");
        let bill = &updated["bill"];
        assert_eq!(bill["paid"], true);
        assert_eq!(bill["total_amount"], 6290);
        let receipt = bill["receipt_url"].as_str().unwrap();
        let (seconds, reading) = receipt.split_once("-r").unwrap();
        assert_eq!(reading, w.reading.id.to_string());
        let seconds: i64 = seconds.parse().unwrap();
        assert!(seconds >= before && seconds <= chrono::Utc::now().timestamp());

        let key = format!("receipts/{TENANT_NAME}/{receipt}");
        let stored = app.files.file(&key).expect("stored receipt");
        assert_eq!(stored.bytes, bytes);
        assert_eq!(stored.content_type.as_deref(), Some(expected_type));
    }
}

#[tokio::test]
async fn upload_refuses_unsupported_receipt_types() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let upload = format!("/api/bills/{}/upload", created["bill"]["id"]);

    for (bytes, declared) in [
        (samples::HEIC, "image/heic"),
        (samples::EXE, "image/jpeg"),
        (b"" as &[u8], "image/jpeg"),
    ] {
        let form =
            upload_form(&w, w.reading.id).file("receipt_file", "receipt.jpg", declared, bytes);
        let (status, body) = app.put_multipart(&upload, Some(&token), form).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{declared}");
        assert_eq!(body, error("Unsupported receipt type"));
    }
    assert!(app.files.keys().is_empty());
}

#[tokio::test]
async fn upload_validates_every_field_before_storing() {
    let app = test_app().await;
    let w = world(&app).await;
    let token = app.admin_token();
    let created = create_bill(&app, &w).await;
    let upload = format!("/api/bills/{}/upload", created["bill"]["id"]);
    let file = |form: Multipart| form.file("receipt_file", "r.jpg", "image/jpeg", samples::JPEG);
    let fields = [
        ("tenant_id", w.tenant.id.to_string()),
        ("reading_id", w.reading.id.to_string()),
        ("room_charges", "5000".to_string()),
        ("electric_charges", "1190".to_string()),
    ];

    for missing in [
        "tenant_id",
        "reading_id",
        "room_charges",
        "electric_charges",
    ] {
        let mut form = Multipart::default();
        for (name, value) in &fields {
            if *name != missing {
                form = form.field(name, value);
            }
        }
        let (status, body) = app.put_multipart(&upload, Some(&token), file(form)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "missing {missing}");
        assert_eq!(body, error(&format!("{missing} is required")));

        let mut form = Multipart::default();
        for (name, value) in &fields {
            let value = if *name == missing { "12.5" } else { value };
            form = form.field(name, value);
        }
        let (status, body) = app.put_multipart(&upload, Some(&token), file(form)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "bad {missing}");
        assert_eq!(
            body,
            error(&format!("{missing} must be an integer, got \"12.5\""))
        );
    }

    for bad in ["{}", "[{\"amount\": 1.5, \"description\": \"x\"}]", "nope"] {
        let form = Multipart::default()
            .field("tenant_id", w.tenant.id)
            .field("reading_id", w.reading.id)
            .field("room_charges", 1)
            .field("electric_charges", 1)
            .field("additional_charges", bad);
        let (status, body) = app.put_multipart(&upload, Some(&token), file(form)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert!(
            error_text(&body).starts_with("additional_charges must be a JSON array"),
            "{body}"
        );
    }

    // A total beyond i32.
    let form = upload_form(&w, w.reading.id).field("electric_charges", i32::MAX);
    let (status, body) = app.put_multipart(&upload, Some(&token), file(form)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, error("The total amount is too large"));

    // A receipt_file part without a filename.
    let form = upload_form(&w, w.reading.id).field("receipt_file", "not a file");
    let (status, body) = app.put_multipart(&upload, Some(&token), form).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        error("receipt_file must be a file part with a filename")
    );

    // The tenant must exist before anything is stored.
    let mut form = upload_form(&w, w.reading.id);
    form = form.field("tenant_id", 999);
    let (status, body) = app.put_multipart(&upload, Some(&token), file(form)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, error("Tenant 999 not found"));

    // Not multipart at all.
    let reply = app
        .request(
            Method::PUT,
            &upload,
            Some(&token),
            Some(("application/json", b"{}".to_vec())),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(reply.json()["error"].is_string());

    assert!(app.files.keys().is_empty(), "nothing was stored");
    let (_, latest) = app
        .get(&format!("/api/bills/{}/bill", w.tenant.id), Some(&token))
        .await;
    assert_eq!(latest, created, "the bill is unchanged");
}

#[tokio::test]
async fn upload_over_10_mib_is_413() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let mut big = samples::JPEG.to_vec();
    big.resize(10 * 1024 * 1024 + 1, 0);
    let form = upload_form(&w, w.reading.id).file("receipt_file", "big.jpg", "image/jpeg", &big);
    let (status, body) = app
        .put_multipart(
            &format!("/api/bills/{}/upload", created["bill"]["id"]),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, error("The upload is larger than 10 MiB"));
    assert!(app.files.keys().is_empty());

    // Just under the limit (with the form around it) is fine.
    let mut fits = samples::JPEG.to_vec();
    fits.resize(10 * 1024 * 1024 - 4096, 0);
    let form = upload_form(&w, w.reading.id).file("receipt_file", "fits.jpg", "image/jpeg", &fits);
    let (status, _) = app
        .put_multipart(
            &format!("/api/bills/{}/upload", created["bill"]["id"]),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn upload_storage_failure_is_502_and_leaves_the_bill() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    app.files.set_failing(true);
    let form =
        upload_form(&w, w.reading.id).file("receipt_file", "r.jpg", "image/jpeg", samples::JPEG);
    let (status, body) = app
        .put_multipart(
            &format!("/api/bills/{}/upload", created["bill"]["id"]),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body, error("File storage is unavailable, try again"));
    app.files.set_failing(false);
    let (_, latest) = app
        .get(
            &format!("/api/bills/{}/bill", w.tenant.id),
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(latest, created);
}

#[tokio::test]
async fn upload_removes_the_stored_receipt_when_the_update_fails() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    // A reading that does not exist passes the pre-upload checks, then fails the update.
    let form = upload_form(&w, 999).file("receipt_file", "r.jpg", "image/jpeg", samples::JPEG);
    let (status, body) = app
        .put_multipart(
            &format!("/api/bills/{}/upload", created["bill"]["id"]),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body["error"].is_string());
    assert!(app.files.keys().is_empty(), "the receipt was removed again");
    let (_, latest) = app
        .get(
            &format!("/api/bills/{}/bill", w.tenant.id),
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(latest, created, "the update was rolled back");
}

// ---------- old receipts are archived ----------

/// Asserts `key` was moved to `archive/<key>` with its bytes and type.
fn assert_archived(app: &TestApp, key: &str, bytes: &[u8], content_type: &str) {
    assert!(app.files.file(key).is_none(), "{key} is no longer live");
    let archived = app
        .files
        .file(&format!("archive/{key}"))
        .unwrap_or_else(|| panic!("{key} is archived"));
    assert_eq!(archived.bytes, bytes);
    assert_eq!(archived.content_type.as_deref(), Some(content_type));
}

/// Gives the bill the receipt `1700000000-r<reading>` with its file stored, as
/// an earlier upload would have; returns the receipt and its storage key.
async fn with_stored_receipt(app: &TestApp, w: &World, bill_id: &Value) -> (String, String) {
    let receipt = format!("1700000000-r{}", w.reading.id);
    let key = format!("receipts/{TENANT_NAME}/{receipt}");
    app.files.insert(&key, samples::JPEG, "image/jpeg");
    let mut body = bill_body(w, w.reading.id);
    body["receipt_url"] = json!(receipt);
    let (status, updated) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&app.admin_token()),
            body,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["bill"]["receipt_url"], json!(receipt));
    (receipt, key)
}

#[tokio::test]
async fn replacing_a_receipt_archives_the_old_file() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let (_, old_key) = with_stored_receipt(&app, &w, &bill_id).await;

    let form =
        upload_form(&w, w.reading.id).file("receipt_file", "r.webp", "image/webp", samples::WEBP);
    let (status, uploaded) = app
        .put_multipart(
            &format!("/api/bills/{bill_id}/upload"),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{uploaded}");
    let receipt = uploaded["bill"]["receipt_url"].as_str().unwrap();
    assert_eq!(
        app.files.keys(),
        vec![
            format!("archive/{old_key}"),
            format!("receipts/{TENANT_NAME}/{receipt}")
        ],
        "the new receipt is live, the old one archived"
    );
    assert_archived(&app, &old_key, samples::JPEG, "image/jpeg");
}

#[tokio::test]
async fn keeping_a_receipt_keeps_its_file() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let (receipt, key) = with_stored_receipt(&app, &w, &bill_id).await;

    // Both updates send the bill's current receipt, as the admin app does.
    let mut body = bill_body(&w, w.reading.id);
    body["receipt_url"] = json!(receipt);
    let (status, _) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&app.admin_token()),
            body,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let form = upload_form(&w, w.reading.id).field("receipt_url", &receipt);
    let (status, updated) = app
        .put_multipart(
            &format!("/api/bills/{bill_id}/upload"),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["bill"]["receipt_url"], json!(receipt));
    assert_eq!(app.files.keys(), vec![key]);
}

#[tokio::test]
async fn clearing_a_receipt_or_deleting_the_bill_archives_its_file() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let (_, key) = with_stored_receipt(&app, &w, &bill_id).await;

    let (status, cleared) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&app.admin_token()),
            bill_body(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["bill"]["paid"], false);
    assert_archived(&app, &key, samples::JPEG, "image/jpeg");

    // The same name again, so the archived copy is replaced by this one.
    with_stored_receipt(&app, &w, &bill_id).await;
    app.files.insert(&key, samples::PNG, "image/png");
    let (status, _) = app
        .delete(&format!("/api/bills/{bill_id}"), Some(&app.admin_token()))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_archived(&app, &key, samples::PNG, "image/png");
    assert_eq!(app.files.keys(), vec![format!("archive/{key}")]);
}

#[tokio::test]
async fn a_receipt_another_bill_still_has_is_kept() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let (receipt, key) = with_stored_receipt(&app, &w, &bill_id).await;

    // A second bill of the same tenant given the same receipt by hand (the JSON API allows it).
    let second_reading = seed_reading(app.db(), &w.tenant, 150, 200).await;
    let mut body = bill_body(&w, second_reading.id);
    body["receipt_url"] = json!(receipt);
    let (status, second) = app
        .post("/api/bills", Some(&app.admin_token()), body.clone())
        .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let second_id = second["bill"]["id"].clone();
    let (status, _) = app
        .put(
            &format!("/api/bills/{second_id}"),
            Some(&app.admin_token()),
            body,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = app
        .delete(&format!("/api/bills/{bill_id}"), Some(&app.admin_token()))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.files.keys(),
        vec![key],
        "the other bill's receipt stays"
    );
}

#[tokio::test]
async fn a_failed_receipt_archive_still_saves_the_bill() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let (_, key) = with_stored_receipt(&app, &w, &bill_id).await;

    app.files.set_failing(true);
    let (status, cleared) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&app.admin_token()),
            bill_body(&w, w.reading.id),
        )
        .await;
    app.files.set_failing(false);
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["bill"]["paid"], false);
    assert_eq!(
        app.files.keys(),
        vec![key],
        "the old file is left at its key (logged)"
    );
}

// ---------- signed links and files ----------

#[tokio::test]
async fn receipt_links_resolve_to_the_stored_file() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let form =
        upload_form(&w, w.reading.id).file("receipt_file", "r.png", "image/png", samples::PNG);
    let (status, uploaded) = app
        .put_multipart(
            &format!("/api/bills/{}/upload", created["bill"]["id"]),
            Some(&app.admin_token()),
            form,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let receipt = uploaded["bill"]["receipt_url"].as_str().unwrap();

    // As the apps request it: the encoded tenant name plus the receipt_url.
    let uri = format!("/api/signed-urls/receipts/Juan%20Dela%20Cruz/{receipt}");
    let own = app.tenant_token(w.tenant.id, TENANT_NAME);
    let (status, body) = app.get(&uri, Some(&own)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["content_type"], "image/png");
    let url = body["url"].as_str().unwrap();
    assert!(
        url.starts_with(&format!(
            "http://api.test/api/files/receipts/Juan%20Dela%20Cruz/{receipt}?expires="
        )),
        "{url}"
    );
    let expires: i64 = url
        .split("expires=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let ttl = expires - chrono::Utc::now().timestamp();
    assert!((595..=600).contains(&ttl), "10-minute links, got {ttl}s");

    let reply = app
        .request(Method::GET, path_and_query(url), None, None)
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, samples::PNG);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "private, max-age=600");
    assert_eq!(reply.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
}

#[tokio::test]
async fn receipt_links_follow_the_permission_table() {
    let app = test_app().await;
    app.files
        .insert("receipts/juan/1700000000-r1", samples::JPEG, "image/jpeg");
    let uri = "/api/signed-urls/receipts/juan/1700000000-r1";

    let (status, body) = app.get(uri, Some(&app.admin_token())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["url"]
            .as_str()
            .unwrap()
            .contains("receipts/juan/1700000000-r1"),
        "{body}"
    );
    assert_eq!(body["content_type"], "image/jpeg");

    let (status, _) = app.get(uri, Some(&app.tenant_token(1, "juan"))).await;
    assert_eq!(status, StatusCode::OK, "own name");
    let (status, body) = app.get(uri, Some(&app.tenant_token(2, "pedro"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("You can only access your own records"));
    let (status, body) = app.get(uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, error("Authentication required"));

    let (status, body) = app
        .get(
            "/api/signed-urls/receipts/juan/1700000001-r1",
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error("Receipt not found"));

    for (bad, field) in [
        (
            "/api/signed-urls/receipts/..%2Fpayments/gcash.png",
            "tenant_name",
        ),
        ("/api/signed-urls/receipts/juan/a%2Fb", "filename"),
        ("/api/signed-urls/receipts/juan/..", "filename"),
        ("/api/signed-urls/receipts/ju%5Can/x", "tenant_name"),
    ] {
        let (status, body) = app.get(bad, Some(&app.admin_token())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(
            body,
            error(&format!("{field} must not contain '/', '\\' or '..'")),
            "{bad}"
        );
    }

    app.files.set_failing(true);
    let (status, _) = app.get(uri, Some(&app.admin_token())).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn payment_links_are_for_any_logged_in_user() {
    let app = test_app().await;
    app.files
        .insert("payments/gcash.png", samples::PNG, "image/png");
    for token in [app.admin_token(), app.tenant_token(1, "juan")] {
        let (status, body) = app
            .get("/api/signed-urls/payments/gcash", Some(&token))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let url = body["url"].as_str().unwrap();
        assert!(
            url.starts_with("http://api.test/api/files/payments/gcash.png?expires="),
            "{url}"
        );
        assert_eq!(body["content_type"], "image/png");
        let reply = app
            .request(Method::GET, path_and_query(url), None, None)
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.body, samples::PNG);
    }

    let (status, body) = app.get("/api/signed-urls/payments/gcash", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, error("Authentication required"));
    let (status, body) = app
        .get("/api/signed-urls/payments/maya", Some(&app.admin_token()))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error("Payment image not found"));
    let (status, body) = app
        .get("/api/signed-urls/payments/..", Some(&app.admin_token()))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, error("name must not contain '/', '\\' or '..'"));
}

// ---------- payment images ----------

fn payment_form(bytes: &[u8]) -> Multipart {
    Multipart::default().file("file", "qr.png", "image/png", bytes)
}

fn payment_list(gcash: bool) -> Value {
    json!([
        { "name": "bpi", "key": "payments/bpi.png", "exists": false },
        { "name": "gcash", "key": "payments/gcash.png", "exists": gcash },
        { "name": "maya", "key": "payments/maya.png", "exists": false },
    ])
}

#[tokio::test]
async fn admin_replaces_payment_images_as_png() {
    let app = test_app().await;
    let token = app.admin_token();
    let (status, body) = app.get("/api/payments", Some(&token)).await;
    assert_eq!((status, body), (StatusCode::OK, payment_list(false)));

    let (status, body) = app
        .put_multipart(
            "/api/payments/gcash",
            Some(&token),
            payment_form(samples::PNG),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({ "name": "gcash", "key": "payments/gcash.png", "exists": true })
    );
    let stored = app.files.file("payments/gcash.png").unwrap();
    assert_eq!(stored.bytes, samples::PNG);
    assert_eq!(stored.content_type.as_deref(), Some("image/png"));

    // Replacing overwrites the same key.
    let mut newer = samples::PNG.to_vec();
    newer.extend_from_slice(b"newer");
    let (status, _) = app
        .put_multipart("/api/payments/gcash", Some(&token), payment_form(&newer))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(app.files.keys(), vec!["payments/gcash.png".to_string()]);
    assert_eq!(app.files.file("payments/gcash.png").unwrap().bytes, newer);

    let (status, body) = app.get("/api/payments", Some(&token)).await;
    assert_eq!((status, body), (StatusCode::OK, payment_list(true)));

    // Tenants see it through the signed link.
    let (status, body) = app
        .get(
            "/api/signed-urls/payments/gcash",
            Some(&app.tenant_token(1, "juan")),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["content_type"], "image/png");
}

#[tokio::test]
async fn payment_uploads_are_admin_only_png_for_known_methods() {
    let app = test_app().await;
    let token = app.admin_token();

    // The declared type is ignored.
    for bytes in [samples::JPEG, samples::WEBP, samples::EXE, b"" as &[u8]] {
        let (status, body) = app
            .put_multipart("/api/payments/gcash", Some(&token), payment_form(bytes))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body, error("Payment images must be PNG files"));
    }

    for name in ["paypal", "gcash.png", "GCASH"] {
        let (status, body) = app
            .put_multipart(
                &format!("/api/payments/{name}"),
                Some(&token),
                payment_form(samples::PNG),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(
            body,
            error(&format!(
                "Unknown payment method {name:?}: expected one of bpi, gcash, maya"
            ))
        );
    }

    let (status, body) = app
        .put_multipart(
            "/api/payments/gcash",
            Some(&token),
            Multipart::default().field("other", "x"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, error("file is required"));

    let mut big = samples::PNG.to_vec();
    big.resize(2 * 1024 * 1024 + 1, 0);
    let (status, body) = app
        .put_multipart("/api/payments/gcash", Some(&token), payment_form(&big))
        .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, error("The upload is larger than 2 MiB"));

    let tenant = app.tenant_token(1, "juan");
    let (status, body) = app.get("/api/payments", Some(&tenant)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("Admin access required"));
    let (status, body) = app
        .put_multipart(
            "/api/payments/gcash",
            Some(&tenant),
            payment_form(samples::PNG),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("Admin access required"));
    let (status, _) = app.get("/api/payments", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = app
        .put_multipart("/api/payments/gcash", None, payment_form(samples::PNG))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    assert!(app.files.keys().is_empty(), "nothing was stored");

    app.files.set_failing(true);
    let (status, _) = app.get("/api/payments", Some(&token)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn file_links_must_be_valid_and_unexpired() {
    let app = test_app().await;
    app.files
        .insert("payments/gcash.png", samples::PNG, "image/png");
    let signer: &FileSigner = &app.state.signer;
    let now = chrono::Utc::now().timestamp();
    let invalid = error("Invalid or expired link");

    let expired = signer.signed_path("payments/gcash.png", now - 1);
    let tampered = signer
        .signed_path("payments/gcash.png", now + 600)
        .replace("gcash", "maya");
    let wrong_expiry = signer.signed_path("payments/gcash.png", now + 600).replace(
        &format!("expires={}", now + 600),
        &format!("expires={}", now + 6000),
    );
    let other_secret =
        FileSigner::new("another-secret").signed_path("payments/gcash.png", now + 600);
    for uri in [
        expired.as_str(),
        tampered.as_str(),
        wrong_expiry.as_str(),
        other_secret.as_str(),
        "/api/files/payments/gcash.png",
        "/api/files/payments/gcash.png?expires=soon&signature=x",
    ] {
        let reply = app.request(Method::GET, uri, None, None).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(reply.json(), invalid, "{uri}");
    }

    // A valid link to a file that is gone.
    let missing = signer.signed_path("payments/maya.png", now + 600);
    let reply = app.request(Method::GET, &missing, None, None).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.json(), error("File not found"));

    // Storage failures are 502.
    let valid = signer.signed_path("payments/gcash.png", now + 600);
    app.files.set_failing(true);
    let reply = app.request(Method::GET, &valid, None, None).await;
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
}

// ---------- repositories ----------

#[tokio::test]
async fn bill_repo_reads_and_writes() {
    let app = test_app().await;
    let w = world(&app).await;
    let db = &app.state.db;

    // test_create_and_get_bill
    let first = seed_bill(app.db(), &w.reading, 1000, 500).await;
    let fetched = bill_repo::get_by_id(app.db(), first.id).await.unwrap();
    assert_eq!(fetched.unwrap().total_amount, 1500);
    assert_eq!(
        bill_repo::get_by_reading_id(app.db(), w.reading.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        first.id
    );

    // test_get_latest_by_tenant_id / test_get_all_by_tenant_id: ties go to the higher id.
    let reading = seed_reading(app.db(), &w.tenant, 150, 200).await;
    let second = seed_bill(app.db(), &reading, 1100, 550).await;
    let latest = bill_repo::get_latest_by_tenant_id(app.db(), w.tenant.id)
        .await
        .unwrap();
    assert_eq!(latest.unwrap().id, second.id);
    let bills = bill_repo::get_all_by_tenant_id(app.db(), w.tenant.id)
        .await
        .unwrap();
    assert_eq!(
        bills.iter().map(|b| b.id).collect::<Vec<_>>(),
        [second.id, first.id]
    );
    assert_eq!(bill_repo::get_all(app.db()).await.unwrap().len(), 2);
    assert!(
        bill_repo::get_all_by_tenant_id(app.db(), 999)
            .await
            .unwrap()
            .is_empty()
    );

    // update_statement changes only the columns that are set.
    let update = bill::ActiveModel {
        total_amount: sea_orm::Set(42),
        ..Default::default()
    };
    db.atomic(vec![bill_repo::update_statement(
        db.backend(),
        first.id,
        update,
    )])
    .await
    .unwrap();
    let updated = bill_repo::get_by_id(app.db(), first.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.total_amount, 42);
    assert_eq!(updated.room_charges, 1000);
    assert_eq!(updated.updated_at, first.updated_at);

    // test_delete_bill
    db.atomic(vec![bill_repo::delete_statement(db.backend(), first.id)])
        .await
        .unwrap();
    assert!(
        bill_repo::get_by_id(app.db(), first.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn additional_charge_repo_reads_and_writes() {
    let app = test_app().await;
    let w = world(&app).await;
    let db = &app.state.db;
    let backend = db.backend();
    let bill = seed_bill(app.db(), &w.reading, 2000, 1000).await;

    // test_create_and_get_additional_charge / test_get_all_and_get_all_by_bill_id
    db.atomic(vec![
        additional_charge_repo::insert_statement(backend, bill.id, 200, "Water"),
        additional_charge_repo::insert_statement(backend, bill.id, 300, "Maintenance"),
    ])
    .await
    .unwrap();
    let charges = additional_charge_repo::get_all_by_bill_id(app.db(), bill.id)
        .await
        .unwrap();
    assert_eq!(charges.len(), 2);
    assert_eq!(charges[0].description, "Water", "oldest first");
    assert_eq!(charges[1].description, "Maintenance");
    assert!(charges[0].id < charges[1].id);

    // A charge on the bill of a reading, written before that bill's id is known.
    let reading = seed_reading(app.db(), &w.tenant, 200, 250).await;
    let other = bill::ActiveModel {
        reading_id: sea_orm::Set(reading.id),
        tenant_id: sea_orm::Set(w.tenant.id),
        room_charges: sea_orm::Set(1),
        electric_charges: sea_orm::Set(1),
        total_amount: sea_orm::Set(2),
        paid: sea_orm::Set(false),
        receipt_url: sea_orm::Set(None),
        ..Default::default()
    };
    db.atomic(vec![
        bill_repo::insert_statement(backend, other),
        additional_charge_repo::insert_for_reading_statement(backend, reading.id, 500, "Late fee"),
    ])
    .await
    .unwrap();
    let other = bill_repo::get_by_reading_id(app.db(), reading.id)
        .await
        .unwrap()
        .unwrap();
    let both = additional_charge_repo::get_all_by_bill_ids(app.db(), &[bill.id, other.id])
        .await
        .unwrap();
    assert_eq!(both.len(), 3);
    let late_fee = both.iter().find(|c| c.bill_id == other.id).unwrap();
    assert_eq!(late_fee.description, "Late fee");

    // test_delete_and_delete_many_by_bill_id
    db.atomic(vec![additional_charge_repo::delete_by_bill_id_statement(
        backend, bill.id,
    )])
    .await
    .unwrap();
    assert!(
        additional_charge_repo::get_all_by_bill_id(app.db(), bill.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        additional_charge_repo::get_all_by_bill_id(app.db(), other.id)
            .await
            .unwrap()
            .len(),
        1,
        "other bills keep their charges"
    );
    assert!(
        additional_charge_repo::get_all_by_bill_ids(app.db(), &[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn read_repos_find_readings_and_tenants() {
    let app = test_app().await;
    let w = world(&app).await;
    let second = seed_reading(app.db(), &w.tenant, 150, 170).await;

    assert_eq!(
        reading_read_repo::get_by_id(app.db(), w.reading.id)
            .await
            .unwrap(),
        Some(w.reading.clone())
    );
    assert_eq!(
        reading_read_repo::get_by_id(app.db(), 999).await.unwrap(),
        None
    );
    let ids: Vec<i32> = (1..=250).collect();
    let mut found = reading_read_repo::get_by_ids(app.db(), &ids).await.unwrap();
    found.sort_by_key(|r| r.id);
    assert_eq!(found, vec![w.reading.clone(), second]);

    assert_eq!(
        tenant_read_repo::get_by_id(app.db(), w.tenant.id)
            .await
            .unwrap(),
        Some(w.tenant.clone())
    );
    assert_eq!(
        tenant_read_repo::get_by_id(app.db(), 999).await.unwrap(),
        None
    );
}

// ---------- payment images (the tenant's proof of payment) ----------

fn payment_proof_form(bytes: &[u8]) -> Multipart {
    Multipart::default().file("payment_file", "payment.bin", "image/png", bytes)
}

async fn upload_payment(
    app: &TestApp,
    bill_id: &Value,
    token: Option<&str>,
    bytes: &[u8],
) -> (StatusCode, Value) {
    app.put_multipart(
        &format!("/api/bills/{bill_id}/payment"),
        token,
        payment_proof_form(bytes),
    )
    .await
}

fn payment_key(bill: &Value) -> String {
    format!(
        "tenant-payments/{TENANT_NAME}/{}",
        bill["bill"]["payment_url"].as_str().expect("a payment_url")
    )
}

#[tokio::test]
async fn tenant_uploads_a_payment_image_to_its_own_bill() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    assert_eq!(created["bill"]["payment_url"], Value::Null);
    let bill_id = created["bill"]["id"].clone();
    let own = app.tenant_token(w.tenant.id, TENANT_NAME);

    let before = chrono::Utc::now().timestamp();
    let (status, updated) = upload_payment(&app, &bill_id, Some(&own), samples::WEBP).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_bill_details(&updated, "payment upload");
    let bill = &updated["bill"];
    assert_eq!(
        bill["paid"], false,
        "a payment image alone does not make the bill paid"
    );
    assert_eq!(bill["receipt_url"], Value::Null);
    assert_eq!(bill["total_amount"], created["bill"]["total_amount"]);
    let payment = bill["payment_url"].as_str().unwrap();
    let (seconds, reading) = payment.split_once("-r").unwrap();
    assert_eq!(reading, w.reading.id.to_string());
    let seconds: i64 = seconds.parse().unwrap();
    assert!(seconds >= before && seconds <= chrono::Utc::now().timestamp());
    let stored = app
        .files
        .file(&payment_key(&updated))
        .expect("stored payment image");
    assert_eq!(stored.bytes, samples::WEBP);
    assert_eq!(stored.content_type.as_deref(), Some("image/webp"));
    assert_eq!(updated["additional_charges"], created["additional_charges"]);
}

#[tokio::test]
async fn payment_uploads_follow_the_permission_table() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let other = seed_tenant(app.db(), w.room.id, "Pedro").await;

    let pedro = app.tenant_token(other.id, "Pedro");
    let (status, body) = upload_payment(&app, &bill_id, Some(&pedro), samples::PNG).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("You can only access your own records"));
    // A token with the bill tenant's name but another id is still someone else.
    let impostor = app.tenant_token(other.id, TENANT_NAME);
    let (status, _) = upload_payment(&app, &bill_id, Some(&impostor), samples::PNG).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = upload_payment(&app, &bill_id, None, samples::PNG).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, error("Authentication required"));
    assert!(app.files.keys().is_empty());

    let (status, body) =
        upload_payment(&app, &json!(999), Some(&app.admin_token()), samples::PNG).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error("Bill 999 not found"));

    // Only admins clear it.
    let uri = format!("/api/bills/{bill_id}/payment");
    let own = app.tenant_token(w.tenant.id, TENANT_NAME);
    let (status, body) = app.delete(&uri, Some(&own)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("Admin access required"));
    let (status, _) = app.delete(&uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn tenant_cannot_change_the_payment_of_a_paid_bill_but_the_admin_can() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    with_stored_receipt(&app, &w, &bill_id).await;
    let own = app.tenant_token(w.tenant.id, TENANT_NAME);

    let (status, body) = upload_payment(&app, &bill_id, Some(&own), samples::PNG).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, error("This bill is already paid"));
    assert_eq!(app.files.keys().len(), 1, "only the receipt is stored");

    let (status, updated) =
        upload_payment(&app, &bill_id, Some(&app.admin_token()), samples::PNG).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["bill"]["paid"], true);
    assert!(app.files.file(&payment_key(&updated)).is_some());
}

#[tokio::test]
async fn payment_upload_refuses_bad_forms_and_types() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let token = app.admin_token();
    let uri = format!("/api/bills/{bill_id}/payment");

    for bytes in [samples::HEIC, samples::EXE] {
        let (status, body) = upload_payment(&app, &bill_id, Some(&token), bytes).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body, error("Unsupported payment image type"));
    }
    let (status, body) = app
        .put_multipart(&uri, Some(&token), Multipart::default().field("other", 1))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, error("payment_file is required"));

    let mut big = samples::JPEG.to_vec();
    big.resize(10 * 1024 * 1024 + 1, 0);
    let (status, body) = upload_payment(&app, &bill_id, Some(&token), &big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, error("The upload is larger than 10 MiB"));

    app.files.set_failing(true);
    let (status, _) = upload_payment(&app, &bill_id, Some(&token), samples::PNG).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    app.files.set_failing(false);

    assert!(app.files.keys().is_empty());
    let (_, latest) = app
        .get(&format!("/api/bills/{}/bill", w.tenant.id), Some(&token))
        .await;
    assert_eq!(latest, created, "the bill is unchanged");
}

#[tokio::test]
async fn replacing_and_clearing_a_payment_archives_the_old_file() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let token = app.admin_token();

    // An earlier upload, as stored then.
    use sea_orm::ConnectionTrait;
    let old_key = format!("tenant-payments/{TENANT_NAME}/1700000000-r{}", w.reading.id);
    app.files.insert(&old_key, samples::JPEG, "image/jpeg");
    app.db()
        .execute_unprepared(&format!(
            "UPDATE bill SET payment_url = '1700000000-r{}' WHERE id = {bill_id}",
            w.reading.id
        ))
        .await
        .unwrap();

    let own = app.tenant_token(w.tenant.id, TENANT_NAME);
    let (status, updated) = upload_payment(&app, &bill_id, Some(&own), samples::PNG).await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    let new_key = payment_key(&updated);
    assert_eq!(
        app.files.keys(),
        vec![format!("archive/{old_key}"), new_key.clone()],
        "the new payment is live, the old one archived"
    );
    assert_archived(&app, &old_key, samples::JPEG, "image/jpeg");

    // An admin bill edit keeps the payment image.
    let (status, edited) = app
        .put(
            &format!("/api/bills/{bill_id}"),
            Some(&token),
            bill_body(&w, w.reading.id),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(
        edited["bill"]["payment_url"],
        updated["bill"]["payment_url"]
    );

    let (status, cleared) = app
        .delete(&format!("/api/bills/{bill_id}/payment"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["bill"]["payment_url"], Value::Null);
    assert_archived(&app, &new_key, samples::PNG, "image/png");
    assert_eq!(app.files.keys().len(), 2, "both payments are archived");

    let (status, body) = app.delete("/api/bills/999/payment", Some(&token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error("Bill 999 not found"));
}

#[tokio::test]
async fn deleting_a_bill_archives_its_payment_image() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let bill_id = created["bill"]["id"].clone();
    let token = app.admin_token();
    let (status, uploaded) = upload_payment(&app, &bill_id, Some(&token), samples::PNG).await;
    assert_eq!(status, StatusCode::OK);
    let key = payment_key(&uploaded);

    let (status, _) = app
        .delete(&format!("/api/bills/{bill_id}"), Some(&token))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_archived(&app, &key, samples::PNG, "image/png");
    assert_eq!(app.files.keys(), vec![format!("archive/{key}")]);
}

#[tokio::test]
async fn payment_links_follow_the_permission_table() {
    let app = test_app().await;
    let w = world(&app).await;
    let created = create_bill(&app, &w).await;
    let (_, updated) = upload_payment(
        &app,
        &created["bill"]["id"],
        Some(&app.admin_token()),
        samples::PNG,
    )
    .await;
    let payment = updated["bill"]["payment_url"].as_str().unwrap();
    // As the apps request it: the encoded tenant name plus the payment_url.
    let uri = format!("/api/signed-urls/tenant-payments/Juan%20Dela%20Cruz/{payment}");

    let own = app.tenant_token(w.tenant.id, TENANT_NAME);
    let (status, body) = app.get(&uri, Some(&own)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["content_type"], "image/png");
    let url = body["url"].as_str().unwrap();
    let reply = app
        .request(Method::GET, path_and_query(url), None, None)
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, samples::PNG);

    let (status, _) = app.get(&uri, Some(&app.admin_token())).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = app.get(&uri, Some(&app.tenant_token(2, "Pedro"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, error("You can only access your own records"));
    let (status, _) = app.get(&uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = app
        .get(
            "/api/signed-urls/tenant-payments/Juan%20Dela%20Cruz/1-r1",
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, error("Payment image not found"));
    let (status, _) = app
        .get(
            "/api/signed-urls/tenant-payments/..%2Freceipts/x",
            Some(&app.admin_token()),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
