# pg-to-d1

One-time move of the production data from the old Postgres database to D1. Node, no dependencies; needs `psql`.

```powershell
node export.mjs <postgres url> C:\src\backups\m18-d1.sql          # read-only on Postgres; output holds tenant data: never in a repo
npx wrangler d1 execute m18-residences --remote --file C:\src\backups\m18-d1.sql
node verify.mjs <postgres url> m18-residences --remote             # row count, max id and money sums per table
```

- `export.mjs` keeps every id and continues each AUTOINCREMENT counter where the Postgres sequence stands (found
  with `pg_get_serial_sequence`; the old Prisma-era sequences are named e.g. `"Rooms_id_seq"`), so ids are never reused.
  Booleans become 0/1, timestamps stay UTC text, and NULL stays distinct from an empty string.
- The target must have the schema already (`wrangler d1 migrations apply`) and be empty.
- Rehearsed on 2026-10-02 against a local and a scratch remote D1: all tables matched.
