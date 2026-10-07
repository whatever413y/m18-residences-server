# dev-seed

Fills the **development** environment (Worker `development-api`, D1 and R2 `m18-residences-dev`) with synthetic
data. No real tenant data is ever used.

- 6 rooms and 6 tenants (`ALPHA` … `FOXTROT`).
- 12 months of readings and bills per tenant. All but the latest are paid, each with a sample receipt image.
- The latest bills cover every status: `ALPHA` and `BRAVO` have a tenant payment image and no receipt
  (**For verification**), `CHARLIE` and `DELTA` are **Paid**, `ECHO` and `FOXTROT` are **Unpaid**.
- Payment methods: BPI, GCash and Maya with synthetic account details and placeholder QR images
  (`payments/{bpi,gcash,maya}.png`; not scannable), and "Sample Bank" without a QR image.

```sh
node seed.mjs           # into an empty dev database (after the D1 migrations)
node seed.mjs --reset   # empty the dev tables first, then seed
node seed.mjs --local --persist-to <dir>   # wrangler's local D1/R2 state instead (used by the e2e screenshots)
```

- **Safety:** remotely, it refuses any database or bucket not named `*-dev`. `--local` only writes wrangler's local
  state in `<dir>` (the default environment's `m18-residences` bindings, as `wrangler dev --persist-to <dir>` reads them).
- **Credentials (remote only):** `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` come from the secrets file
  `$M18_SECRETS_FILE` (default `../../../m18-residences-infra/.env`).
- **Wrangler:** `$WRANGLER_JS`, or the e2e suite's pinned copy.
- **`--reset` and the bucket:** re-seeding overwrites the same receipt and QR keys, but files uploaded through the dev
  admin app stay in the bucket. Without `--reset`, BPI, GCash and Maya (ids 1–3) are set back to the seeded details.
