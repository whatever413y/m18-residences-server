-- The full R2 key of each bill's receipt and payment image, written when the file is stored, so renaming a tenant or
-- moving a bill to another tenant no longer loses its files (the old keys were rebuilt from the current tenant name).
-- `receipt_url` / `payment_url` keep the file name: they still say whether there is a file. Existing files get the key
-- they were stored under (their tenant's name now, which is the name at upload time unless it was renamed since).
ALTER TABLE bill ADD COLUMN receipt_key TEXT;
ALTER TABLE bill ADD COLUMN payment_key TEXT;

UPDATE bill
SET receipt_key = 'receipts/' || (SELECT name FROM tenant WHERE tenant.id = bill.tenant_id) || '/' || receipt_url
WHERE receipt_url IS NOT NULL AND receipt_url <> '';

UPDATE bill
SET payment_key = 'tenant-payments/' || (SELECT name FROM tenant WHERE tenant.id = bill.tenant_id) || '/' || payment_url
WHERE payment_url IS NOT NULL AND payment_url <> '';
