# receipts-backfill

One-time conversion of the receipts uploaded before the admin app converted images itself (P7 of the
Workers move). Each JPEG, PNG, GIF or AVIF under `receipts/` is re-encoded the way the admin app does it
(long edge at most 1600 px, WebP quality 80, EXIF orientation applied) and replaced **only if smaller**.
WebP and PDF files are left alone. Keys stay the same, so the database is untouched.

```powershell
cd tools/receipts-backfill
npm ci
node backfill.mjs            # dry run: downloads, converts in memory, reports sizes; writes nothing
node backfill.mjs --apply    # converts for real
```

- **Backup:** before a receipt is overwritten, the original is copied to `receipts-originals/<key>` (and the
  copy's size checked). The bucket's lifecycle rule (`m18-residences-infra/cloudflare.tf`) deletes those
  copies 30 days later. To restore one, copy it back over `<key>` with its original content type.
- **Credentials:** `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN` (needs R2 Edit) are read from the
  secrets file `$M18_SECRETS_FILE` (default `../../../m18-residences-infra/.env`) and used as S3 credentials
  (token id / SHA-256 of the token). Nothing is printed or stored.
- **Bucket:** `$R2_BUCKET`, default `m18-residences`.
- **Reruns** are safe: converted receipts are WebP and get skipped. A failure is reported per object and
  makes the run exit 1.
- Receipts hold tenant data: don't copy them into a repo.
