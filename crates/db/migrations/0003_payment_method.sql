-- How tenants pay (banks, e-wallets), managed by the admin. `image_key` is the R2 key of the method's QR code
-- (a PNG; NULL = none). Seeded with the three methods the apps had built in, at their existing keys
-- `payments/<name>.png`; images uploaded later get new keys `payments/<id>-<unix ms>.png`.
CREATE TABLE payment_method (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    name           TEXT    NOT NULL,
    account_name   TEXT,
    account_number TEXT,
    sort_order     INTEGER NOT NULL DEFAULT 0,
    image_key      TEXT,
    created_at     TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX payment_method_name_key ON payment_method (name COLLATE NOCASE);

INSERT INTO payment_method (id, name, sort_order, image_key) VALUES
    (1, 'BPI', 1, 'payments/bpi.png'),
    (2, 'GCash', 2, 'payments/gcash.png'),
    (3, 'Maya', 3, 'payments/maya.png');
