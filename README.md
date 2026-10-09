# M18 Residences Server

The M18 Residences API: rooms, tenants, electricity readings, bills, receipts, tenants' payment images and the payment methods (with QR images). Rust (Axum 0.8 + SeaORM 2.0)
running as one Cloudflare Worker (https://api.m18-residences.workers.dev) on a D1 database and an R2 bucket.

## Layout

```sh
m18-residences-server/
├── Cargo.toml            # Workspace + the Worker crate (cdylib for the Worker, rlib for native tests)
├── wrangler.jsonc        # Worker "api": D1 binding DB, R2 binding FILES, vars, build command
├── .dev.vars.example     # Local config for `wrangler dev` (copy to .dev.vars)
├── src/
│   ├── app.rs            # Router: the three domains, /, /health, CORS, AppState
│   ├── worker_entry.rs   # Worker fetch handler (wasm only): bindings + config → app
│   ├── accounts/         # /api/auth: admin and tenant login, token validation
│   ├── property/         # /api/rooms, /api/tenants, /api/electricity-readings
│   └── billing/          # /api/bills, /api/payment-methods, /api/signed-urls, /api/files (receipts, payment and QR images)
├── crates/
│   ├── shared_rs/        # auth, errors, config, CORS, file storage + signed links, extractors, logging
│   └── db/               # entities, D1 adapter, migrations/*.sql (the schema's only source)
├── tests/                # native integration tests (in-memory SQLite + in-memory file store)
└── tools/                # Node tools: dev-seed (synthetic dev data), receipts-backfill (the one-off WebP conversion)
```

Each domain folder has `routes/ → handlers/ → services/ → repository/` and could be moved into its own Worker:
it only uses itself, `crate::app` and the shared crates, writes only its own tables, and reads other tables
through its own `*_read_repo.rs`. `tests/boundaries.rs` checks these rules.

## Local development

Prerequisites: Rust (stable) with `rustup target add wasm32-unknown-unknown`, `cargo install worker-build`,
Node 22 (wrangler runs through `npx`). No database server.

```sh
cp .dev.vars.example .dev.vars                                       # local secrets and allowed origins
npx wrangler@4.145.0 d1 migrations apply m18-residences --local     # local D1 schema (in .wrangler/)
npx wrangler@4.145.0 dev --port 50000                               # builds the Worker and serves it
curl http://localhost:50000/health                                  # {"status":"ok","version":null}
```

Local D1 and R2 live in `.wrangler/` (gitignored). The Flutter apps run on ports 50001 (admin) and 50002 (tenant),
the origins `.dev.vars` allows.

Configuration: `ALLOWED_ORIGINS` (comma-separated), `JWT_SECRET`, `ADMIN_USERNAME`, `ADMIN_PASSWORD`,
`TURNSTILE_SECRET` (the Turnstile widget's secret; locally Cloudflare's always-pass test secret, see
`.dev.vars.example`). If any is missing, every request fails with a 500 that names it. In production
`ALLOWED_ORIGINS` is in `wrangler.jsonc`, the others are Worker secrets (`npx wrangler@4.145.0 secret put <NAME>`).
The binding `LOGIN_RATE_LIMITER` (Workers Rate Limiting, in `wrangler.jsonc`) limits the logins; `wrangler dev`
simulates it locally.

## Database

D1 (SQLite). The schema is `crates/db/migrations/*.sql`, mirrored by `crates/db/src/entities/` and
`crates/db/src/schema.rs`. A change is a new numbered file; an applied migration is never edited.

- Locally: `npx wrangler@4.145.0 d1 migrations apply m18-residences --local`.
- Production: the **Migrate production database** workflow (`migrate.yml`; `list`, then `apply` after your
  approval), before merging code that needs the change.

D1 has no interactive transactions: writes that must happen together go through `db.atomic(statements)`, one
D1 batch.

## Performance

The Workers Free plan allows 10 ms CPU per request, but doesn't enforce it strictly. Measured on 2026-10-05 on the
dev Worker with `wrangler tail`, `GET /api/bills` (every bill with its reading and charges), median CPU:

| Bills | CPU |
|---|---|
| 115 (production today) | 7 ms |
| 500 | 23 ms |
| 1,150 | 47–53 ms (a few requests cut off with error 1102) |

The cost grows with the number of rows. Most of it is D1 building the row objects and their conversion into wasm,
not our code. The D1 adapter (`crates/db/src/d1.rs`) reads rows without per-row maps, looks up column types once per
query and parses timestamps by hand. Two alternatives were measured and not adopted: `JSON.stringify` +
`TextEncoder` + serde_json (faster at 1,150 bills, slower at 500), and D1's `raw()` (more CPU inside D1's own
JavaScript). So the admin app loads fewer rows: `GET /api/bills?since=…` (the last 12 months plus every open bill),
and a year, tenant or room only when the Billing page asks for it (indexes in migration 0005).

Measured again on 2026-10-08 on the dev Worker with 1,200 bills (200 months of 6 tenants), median CPU:

| Request | Rows | CPU |
|---|---|---|
| `GET /api/bills` (every bill) | 1,200 | 60 ms |
| `GET /api/bills?since=<12 months back>` | 72 | 6 ms |
| `GET /api/bills?year=2026` | 60 | 6 ms |
| `GET /api/bills?tenant_id=1` / `?room_id=2` | 200 | 12 ms |
| `GET /api/bills/years` | 2 | 3 ms |
| `GET /api/electricity-readings` (every reading) | 1,200 | 13 ms |

The readings list still loads everything; at today's size it costs a few ms.

## Tests

```sh
cargo test
```

Runs everything natively, without Cloudflare: the domains through the real router on in-memory SQLite with the
same migrations, plus unit tests in the crates. The contract fixtures that `m18_residences_shared` tests against:

```powershell
$env:FIXTURES_OUT='C:\dev\shared-packages\packages\m18_residences_shared\test\fixtures'; cargo test export_contract_fixtures -- --ignored
```

`worker-build --release` checks the Worker build itself.

## API

| Route | Who |
|---|---|
| `GET /`, `GET /health` (`{status, version}`) | anyone |
| `POST /api/auth/admin-login`, `/login` (tenant, by name in any case), `/validate-token` | anyone |
| `GET /api/files/{*key}` | anyone with a valid signed link (10 minutes) |
| `GET /api/tenants/{id}`, `GET /api/bills/{tenant_id}/bill`, `GET /api/bills/{tenant_id}/bills` | admin, or that tenant |
| `GET /api/signed-urls/bills/{id}/receipt`, `GET /api/signed-urls/bills/{id}/payment` | admin, or the bill's tenant |
| `PUT /api/bills/{id}/payment` | admin, or the bill's tenant until the bill has a receipt (then 409) |
| `GET /api/payment-methods`, `GET /api/signed-urls/payment-methods/{id}` | any logged-in user |
| everything else under `/api/rooms`, `/api/tenants`, `/api/electricity-readings`, `/api/bills`, `/api/payment-methods` | admin |

Errors are JSON `{"error": "..."}` (403 for a tenant token on an admin route, 409 for a conflict, 404 for a
missing record). JSON bodies are at most 64 KiB (413); uploads have their own limits. Bills come back as
`{bill, additional_charges, reading}`.

Logins: both need a `turnstile_token` (Cloudflare Turnstile; missing → the same 400 as a rejected one). Before the
credentials are checked: at most 10 attempts a minute per client IP and login (`429` with `Retry-After: 60`; if the
limiter itself fails, the attempt goes through and is logged), then the token (`400` "Verification
failed. Please try again."; if Cloudflare can't be reached, the attempt goes through and is logged: an outage must
not lock everyone out, and the rate limit still applies). Tenants log in with their
name in any case, without surrounding spaces; names are unique in any case (migration 0006), trimmed, 1–64
characters, without control characters, `/`, `\` or `..`.

Bills: `GET /api/bills` returns every bill, or with `?since=YYYY-MM-DD` (created on or after that day, plus every
bill without a receipt), `year=`, `tenant_id=`, `room_id=` (all that are given must match; a bad value is a 400);
`GET /api/bills/years` lists the years with bills, newest first. Readings must not be negative or go down (400).

Receipts: `PUT /api/bills/{id}/upload` (multipart, at most 10 MiB) accepts JPEG, PNG, WebP, GIF, AVIF and PDF,
checked by their bytes, and stores `receipts/<tenant name>/<unix ts>-r<reading id>` after the bill, tenant and
reading are checked. The admin app converts photos to WebP before uploading. Each bill records the full key of its
receipt and payment image (`receipt_key`, `payment_key`; migration 0004, never sent to clients), so renaming a
tenant or moving a bill keeps its files; the bill-id links use them. A JSON update's `receipt_url` may only keep or
clear the receipt (400 otherwise), and applies only if the receipt is still the one the update was based on (409
"This bill changed meanwhile. Reload and try again." otherwise, nothing written). Once a bill no longer points at a receipt (replaced, cleared, or the bill
deleted), that file is moved to `archive/<key>` in R2 (copied, then the original deleted; kept forever), after the
database write; a failed archive leaves the file at its key and records it in `file_cleanup` (migration 0007),
which the Worker's daily scheduled run (`triggers.crons`, 02:00 Manila) retries, at most 50 files a run. Locally:
`npx wrangler@4.145.0 dev --test-scheduled`, then `curl "http://localhost:50000/__scheduled?cron=0+18+*+*+*"`.

Payment images (the tenant's proof of payment, optional): `PUT /api/bills/{id}/payment` (multipart part
`payment_file`, at most 10 MiB, the same types as receipts) stores `tenant-payments/<tenant name>/<unix ts>-r<reading id>`
and sets only the bill's `payment_url` (for a tenant only while the bill has no receipt, checked in the same
statement; a receipt attached meanwhile wins with a 409 and the upload is removed); `DELETE /api/bills/{id}/payment`
(admin) clears it. The replaced or cleared
file, and a deleted bill's, is archived the same way (`archive/tenant-payments/...`) after the database write. `paid` still means "has a receipt"; the
apps show **Unpaid** (neither), **For verification** (payment, no receipt) or **Paid** (receipt).

Payment methods (table `payment_method`, migration 0003; seeded with BPI, GCash and Maya at their old keys
`payments/<name>.png`): `GET /api/payment-methods` lists them in order as
`{id, name, account_name, account_number, sort_order, has_image}`; the admin adds (`POST`, JSON
`{name, account_name?, account_number?, sort_order?}`; a new one goes last), edits (`PUT /{id}`; the place stays
unless given) and deletes (`DELETE /{id}`) them. Names are unique in any case (409), at most 40 characters; account
name at most 80, number at most 40; blank account fields are stored as NULL. The QR image: `PUT /{id}/image`
(multipart part `file`, at most 2 MiB, PNG only, checked by its bytes; the admin app converts the picked image first)
stores `payments/<id>-<unix ms>.png` (a new key per upload, so renames never move files and caches never show an
old image), `DELETE /{id}/image` clears it; the image a method no longer points at (replaced, removed, method
deleted) is archived like receipts. `GET /api/signed-urls/payment-methods/{id}` links to it (404 without one).

Signed-URL responses are `{url, content_type}`; the URL points at `/api/files/...` on this Worker, which streams the
file from R2.

## Development environment

The `development` branch runs as its own Worker, **`development-api`** (https://development-api.m18-residences.workers.dev),
from `env.development` in `wrangler.jsonc`: its own D1 and R2 (`m18-residences-dev`, synthetic data from
[`tools/dev-seed`](tools/dev-seed/README.md)), its own secrets (`wrangler secret put <NAME> --env development`: the
same admin login as production, `ADMIN_USERNAME`/`ADMIN_PASSWORD` in the infra `.env`, and its own random
`JWT_SECRET`, which lives only in Cloudflare), and CORS for the apps' preview links (`https://*-admin…`, `https://*-my…`,
a `*` standing for one DNS label). The production Worker has `preview_urls: false`: a preview link would use the
production data.

## CI/CD

- `test.yml`: PRs to `main` → fmt, clippy (native and wasm32), tests, `worker-build`.
- `development.yml`: pushes to `development` → the same checks → D1 migrations applied to `m18-residences-dev` →
  deploy `development-api` (production untouched; `live` not moved).
- `deploy.yml`: pushes to `main` → the same checks → the browser e2e suite (`shared-e2e`) with this commit and
  the apps as they are live → `wrangler deploy` → waits for `/health` to report the commit → moves the `live` tag.
- `migrate.yml`: production D1 migrations, manual, with approval.

The workflows come from [whatever413y/.github](https://github.com/whatever413y/.github).
