# M18 Residences Server

The M18 Residences API: rooms, tenants, electricity readings, bills and receipts. Rust (Axum 0.8 + SeaORM 2.0)
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
│   └── billing/          # /api/bills, /api/signed-urls, /api/files (receipts and payment images)
├── crates/
│   ├── shared_rs/        # auth, errors, config, CORS, file storage + signed links, extractors, logging
│   └── db/               # entities, D1 adapter, migrations/*.sql (the schema's only source)
├── tests/                # native integration tests (in-memory SQLite + in-memory file store)
└── tools/                # one-off Node tools: pg-to-d1 (data move), receipts-backfill (WebP conversion)
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

Configuration: `ALLOWED_ORIGINS` (comma-separated), `JWT_SECRET`, `ADMIN_USERNAME`, `ADMIN_PASSWORD`. If any is
missing, every request fails with a 500 that names it. In production `ALLOWED_ORIGINS` is in `wrangler.jsonc`,
the others are Worker secrets (`npx wrangler@4.145.0 secret put <NAME>`).

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
JavaScript). The lever left is to load fewer rows per request (paging, or one year at a time in the admin billing
page).

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
| `POST /api/auth/admin-login`, `/login` (tenant, by name), `/validate-token` | anyone |
| `GET /api/files/{*key}` | anyone with a valid signed link (10 minutes) |
| `GET /api/tenants/{id}`, `GET /api/bills/{tenant_id}/bill`, `GET /api/bills/{tenant_id}/bills` | admin, or that tenant |
| `GET /api/signed-urls/receipts/{name}/{file}` | admin, or the tenant with that name |
| `GET /api/signed-urls/payments/{name}` | any logged-in user |
| everything else under `/api/rooms`, `/api/tenants`, `/api/electricity-readings`, `/api/bills` | admin |

Errors are JSON `{"error": "..."}` (403 for a tenant token on an admin route, 409 for a conflict, 404 for a
missing record). Bills come back as `{bill, additional_charges, reading}`.

Receipts: `PUT /api/bills/{id}/upload` (multipart, at most 10 MiB) accepts JPEG, PNG, WebP, GIF, AVIF and PDF,
checked by their bytes. The admin app converts photos to WebP before uploading. Signed-URL responses are
`{url, content_type}`; the URL points at `/api/files/...` on this Worker, which streams the file from R2.

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
