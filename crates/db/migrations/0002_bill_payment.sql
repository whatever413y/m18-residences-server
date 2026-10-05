-- The tenant's proof of payment (file name under `tenant-payments/<tenant name>/`),
-- uploaded by the tenant or the admin. NULL = none. `paid` still means "has a receipt".
ALTER TABLE bill ADD COLUMN payment_url TEXT;
