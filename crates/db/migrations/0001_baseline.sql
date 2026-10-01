-- Baseline schema: the SQLite (D1) translation of the Postgres baseline
-- (m20250820_010340_baseline). Same tables, columns, defaults, unique index
-- names and foreign-key rules. AUTOINCREMENT keeps ids from ever being reused,
-- as with Postgres SERIAL. Timestamps are UTC text ('YYYY-MM-DD HH:MM:SS').

CREATE TABLE room (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    rent       INTEGER NOT NULL,
    created_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX rooms_name_key ON room (name);

CREATE TABLE tenant (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id    INTEGER NOT NULL REFERENCES room (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    name       TEXT    NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    join_date  TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX tenants_name_key ON tenant (name);

CREATE TABLE electricity_reading (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id    INTEGER NOT NULL REFERENCES tenant (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    room_id      INTEGER NOT NULL REFERENCES room (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    prev_reading INTEGER NOT NULL,
    curr_reading INTEGER NOT NULL,
    consumption  INTEGER NOT NULL,
    created_at   TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE bill (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    reading_id       INTEGER NOT NULL REFERENCES electricity_reading (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    tenant_id        INTEGER NOT NULL REFERENCES tenant (id) ON UPDATE CASCADE ON DELETE RESTRICT,
    room_charges     INTEGER NOT NULL DEFAULT 0,
    electric_charges INTEGER NOT NULL DEFAULT 0,
    total_amount     INTEGER NOT NULL,
    receipt_url      TEXT,
    paid             INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX "bills_readingId_key" ON bill (reading_id);

CREATE TABLE additional_charge (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    bill_id     INTEGER NOT NULL REFERENCES bill (id) ON UPDATE CASCADE ON DELETE CASCADE,
    amount      INTEGER NOT NULL DEFAULT 0,
    description TEXT    NOT NULL,
    created_at  TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
