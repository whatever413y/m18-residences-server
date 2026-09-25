# M18 Residences Server

A backend service for managing room rental, tenants, bills, and electricity readings built with Rust and Axum.

## Features

- Tenant and room management
- Bill generation and tracking
- Electricity reading records
- JWT-based authentication (admin and tenant)
- Cloudflare R2 file uploads and signed URLs for receipts.
- RESTful API endpoints

## Project Structure

```sh
m18-residences-server/
├── .cargo/config.toml         # Runs tests one at a time (they share the test DB)
├── .github/workflows/test.yml # CI: migrate a Postgres service, clippy, cargo test
├── migration/                 # Database schema migrations (SeaORM migrator crate)
├── src/
│   ├── main.rs                # Entry point: env, DB, R2, server + graceful shutdown
│   ├── lib.rs                 # Library root (shared logic, exports)
│   ├── app.rs                 # Router: routes, JWT protection, global layers
│   ├── entities/              # Database models (SeaORM entities)
│   ├── handlers/              # HTTP request handlers (Axum)
│   ├── middleware/            # Authentication, CORS, etc.
│   ├── repository/            # Database access logic
│   ├── routes/                # Route definitions
│   ├── services/              # Business logic, integrations (e.g., S3, JWT)
├── tests/
│   ├── common/mod.rs          # Test DB helpers (get_test_db, reset_table)
│   ├── repository/            # Repository tests (one test binary)
│   ├── api.rs                 # API end-to-end tests through the real router
├── Cargo.toml                 # Rust package manifest
├── Cargo.lock                 # Locked dependency versions (committed)
├── .env.example               # Example env file for reference
├── .env.test.example          # Example test env file for reference
├── README.md                  # Project documentation
```

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- PostgreSQL database
- Cloudflare R2 account (for file storage)

### Setup

1. Clone the repository:

   ```sh
   git clone https://github.com/yourusername/m18-residences-server.git
   cd m18-residences-server
   ```

2. Copy `.env.example` to `.env` and fill in your configuration:

   - Localhost origin URLs for browser-based CORS (LOCALHOST_URL, comma-separated — admin app on 50001, tenant app on 50002)
   - Database connection string
   - JWT secret
   - Admin credentials
   - Cloudflare R2 credentials

3. Run database migrations.

   ```sh
   cargo install sea-orm-cli
   sea-orm-cli migrate up
   ```

4. Build and run the server:

   ```sh
   cargo build
   cargo run
   ```

The server will start on the port specified in your `.env` file (default 50000).

## Testing

- `cargo test --lib` — unit tests only (CORS origin parsing); no database needed.
- `cargo test` — everything, including the repository tests (`tests/repository/`) and the API end-to-end tests (`tests/api.rs`, requests through the real router in memory). These need:
  1. `.env.test` — copy `.env.test.example`, point `TEST_DATABASE_URL` at a dedicated test database (the tests TRUNCATE every table) and keep the test-only JWT/admin/CORS values.
  2. A migrated `m18_test` database:

     ```sh
     cd migration
     cargo run -- up -u "postgresql://postgres:password@localhost:5432/m18_test"
     ```

  The DB tests share that database, so `.cargo/config.toml` sets `RUST_TEST_THREADS=1` (tests inside each binary run one at a time).
- API contract fixtures — the JSON responses the Flutter apps consume (JWTs replaced by `<token>`, signed URLs by `<signed-url>`), written to `FIXTURES_OUT` (relative paths resolve from the repository root):

  ```powershell
  $env:FIXTURES_OUT='C:\path\to\fixtures'; cargo test export_contract_fixtures -- --ignored
  ```

CI (`.github/workflows/test.yml`) runs the migrations, `cargo clippy --all-targets` and `cargo test` against a Postgres 18 service on every push and pull request to `main` and `update`.

## API Endpoints

- `/api/auth` - Authentication routes (admin and tenant login, token validation)
- `/api/rooms` - Room management (CRUD)
- `/api/tenants` - Tenant management (CRUD)
- `/api/electricity-readings` - Electricity readings (CRUD)
- `/api/bills` - Bill management (CRUD, file upload)
- `/api/signed-urls` - Generate signed URLs for receipts and payments

All routes except `/`, `/health` and `/api/auth` require JWT authentication.
