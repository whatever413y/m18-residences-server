# dev-seed

Fills the **development** environment (Worker `development-api`, D1 and R2 `m18-residences-dev`) with synthetic
data. No real tenant data is ever used.

- 6 rooms and 6 tenants (`ALPHA` … `FOXTROT`).
- 12 months of readings and bills per tenant. All but the latest are paid, each with a sample receipt image.
- Placeholder payment QR images (`payments/{bpi,gcash,maya}.png`; not scannable).

```sh
node seed.mjs           # into an empty dev database (after the D1 migrations)
node seed.mjs --reset   # empty the dev tables first, then seed
```

- **Safety:** it refuses any database or bucket not named `*-dev`.
- **Credentials:** `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` come from the secrets file `$M18_SECRETS_FILE`
  (default `../../../m18-residences-infra/.env`).
- **Wrangler:** `$WRANGLER_JS`, or the e2e suite's pinned copy.
- **`--reset` and the bucket:** re-seeding overwrites the same receipt keys, but receipts uploaded through the dev
  admin app stay in the bucket.
