//! Test harness. Every test gets its own app over a fresh in-memory SQLite
//! database (the D1 migrations applied) and an in-memory file store, so tests
//! are independent and run in parallel. Each test binary uses only part of
//! this module, hence the `dead_code` allowance.
#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use chrono::Utc;
use m18_residences_db::entities::{bill, electricity_reading, room, tenant};
use m18_residences_server::app::{AppState, app};
use m18_residences_shared_rs::{
    auth::{ADMIN_ROLE, Claims},
    config::Config,
    files::MemoryFileStore,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::Value;
use tower::ServiceExt;

pub const JWT_SECRET: &str = "test-jwt-secret";
pub const ADMIN_USERNAME: &str = "test-admin";
pub const ADMIN_PASSWORD: &str = "test-password";
pub const ADMIN_ORIGIN: &str = "http://localhost:50001";
pub const TENANT_ORIGIN: &str = "http://localhost:50002";

pub fn test_config() -> Config {
    Config::from_lookup(|name| {
        Some(
            match name {
                "JWT_SECRET" => JWT_SECRET,
                "ADMIN_USERNAME" => ADMIN_USERNAME,
                "ADMIN_PASSWORD" => ADMIN_PASSWORD,
                "ALLOWED_ORIGINS" => "http://localhost:50001,http://localhost:50002",
                _ => return None,
            }
            .to_string(),
        )
    })
    .expect("test configuration")
}

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    pub files: Arc<MemoryFileStore>,
}

/// A response: status, headers and raw body.
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Reply {
    /// The body as JSON (`Value::Null` when empty).
    pub fn json(&self) -> Value {
        if self.body.is_empty() {
            return Value::Null;
        }
        serde_json::from_slice(&self.body).unwrap_or_else(|err| {
            panic!(
                "expected a JSON body ({err}), got {}: {}",
                self.status,
                String::from_utf8_lossy(&self.body)
            )
        })
    }
}

pub async fn test_app() -> TestApp {
    let db = m18_residences_db::sqlite::connect_in_memory()
        .await
        .expect("in-memory database");
    let files = Arc::new(MemoryFileStore::default());
    let state = AppState::new(
        db,
        files.clone(),
        test_config(),
        Some("test-version".into()),
    );
    TestApp {
        router: app(state.clone()),
        state,
        files,
    }
}

impl TestApp {
    pub fn db(&self) -> &DatabaseConnection {
        self.state.db.conn()
    }

    /// Sends one request through the router.
    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<(&str, Vec<u8>)>,
    ) -> Reply {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "api.test");
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match body {
            Some((content_type, bytes)) => {
                request = request.header(header::CONTENT_TYPE, content_type);
                Body::from(bytes)
            }
            None => Body::empty(),
        };
        let response = self
            .router
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        Reply {
            status,
            headers,
            body,
        }
    }

    /// A JSON request; returns the status and the JSON body (`Value::Null` when empty).
    pub async fn send(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let body = body.map(|json| ("application/json", json.to_string().into_bytes()));
        let reply = self.request(method, uri, token, body).await;
        (reply.status, reply.json())
    }

    pub async fn get(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.send(Method::GET, uri, token, None).await
    }

    pub async fn post(&self, uri: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
        self.send(Method::POST, uri, token, Some(body)).await
    }

    pub async fn put(&self, uri: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
        self.send(Method::PUT, uri, token, Some(body)).await
    }

    pub async fn delete(&self, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        self.send(Method::DELETE, uri, token, None).await
    }

    /// A `multipart/form-data` PUT.
    pub async fn put_multipart(
        &self,
        uri: &str,
        token: Option<&str>,
        form: Multipart,
    ) -> (StatusCode, Value) {
        let (content_type, body) = form.encode();
        let reply = self
            .request(Method::PUT, uri, token, Some((&content_type, body)))
            .await;
        (reply.status, reply.json())
    }

    /// A CORS preflight for `GET path` from `origin`.
    pub async fn preflight(&self, path: &str, origin: &str) -> Reply {
        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri(path)
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        Reply {
            status,
            headers,
            body: Vec::new(),
        }
    }

    /// A valid admin token, minted directly (independent of the accounts routes).
    pub fn admin_token(&self) -> String {
        self.token(Claims {
            id: None,
            name: Some(ADMIN_USERNAME.into()),
            role: Some(ADMIN_ROLE.into()),
            exp: expires_in(3600),
        })
    }

    /// A valid token for the tenant `id` / `name`, minted directly.
    pub fn tenant_token(&self, id: i32, name: &str) -> String {
        self.token(Claims {
            id: Some(id),
            name: Some(name.into()),
            role: None,
            exp: expires_in(1200),
        })
    }

    pub fn token(&self, claims: Claims) -> String {
        self.state.jwt.issue(&claims).unwrap()
    }
}

fn expires_in(seconds: i64) -> usize {
    (Utc::now().timestamp() + seconds) as usize
}

/// A multipart form: text fields plus an optional file part.
#[derive(Default)]
pub struct Multipart {
    fields: Vec<(String, String)>,
    file: Option<(String, String, String, Vec<u8>)>,
}

impl Multipart {
    pub fn field(mut self, name: &str, value: impl ToString) -> Self {
        self.fields.push((name.into(), value.to_string()));
        self
    }

    /// The file part `name`, with its filename, declared content type and bytes.
    pub fn file(mut self, name: &str, filename: &str, content_type: &str, bytes: &[u8]) -> Self {
        self.file = Some((
            name.into(),
            filename.into(),
            content_type.into(),
            bytes.to_vec(),
        ));
        self
    }

    pub fn encode(&self) -> (String, Vec<u8>) {
        const BOUNDARY: &str = "m18-residences-test-boundary";
        let mut body = Vec::new();
        for (name, value) in &self.fields {
            body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        if let Some((name, filename, content_type, bytes)) = &self.file {
            body.extend_from_slice(
                format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
        (format!("multipart/form-data; boundary={BOUNDARY}"), body)
    }
}

/// Minimal files of each kind, recognizable by their first bytes.
pub mod samples {
    pub const JPEG: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00,
    ];
    pub const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    pub const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00";
    pub const WEBP: &[u8] = b"RIFF\x1a\0\0\0WEBPVP8 ";
    pub const AVIF: &[u8] = b"\0\0\0\x1cftypavif\0\0\0\0avifmif1miaf";
    pub const HEIC: &[u8] = b"\0\0\0\x18ftypheic\0\0\0\0mif1heic";
    pub const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";
    pub const EXE: &[u8] = b"MZ\x90\0\x03\0\0\0";
}

// ---- Seeding rows directly (independent of the domain routes) ----

pub async fn seed_room(db: &DatabaseConnection, name: &str, rent: i32) -> room::Model {
    room::ActiveModel {
        name: Set(name.into()),
        rent: Set(rent),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

pub async fn seed_tenant(db: &DatabaseConnection, room_id: i32, name: &str) -> tenant::Model {
    tenant::ActiveModel {
        room_id: Set(room_id),
        name: Set(name.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

pub async fn seed_reading(
    db: &DatabaseConnection,
    tenant: &tenant::Model,
    prev: i32,
    curr: i32,
) -> electricity_reading::Model {
    electricity_reading::ActiveModel {
        tenant_id: Set(tenant.id),
        room_id: Set(tenant.room_id),
        prev_reading: Set(prev),
        curr_reading: Set(curr),
        consumption: Set(curr - prev),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

/// A bill row without charges or receipt.
pub async fn seed_bill(
    db: &DatabaseConnection,
    reading: &electricity_reading::Model,
    room_charges: i32,
    electric_charges: i32,
) -> bill::Model {
    bill::ActiveModel {
        reading_id: Set(reading.id),
        tenant_id: Set(reading.tenant_id),
        room_charges: Set(room_charges),
        electric_charges: Set(electric_charges),
        total_amount: Set(room_charges + electric_charges),
        receipt_url: Set(None),
        paid: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}
