-- Files whose move to `archive/<key>` failed after the database stopped pointing at them (a replaced, cleared or
-- deleted receipt, payment image or QR image). The Worker's daily scheduled run retries them; a row is deleted once
-- the file is archived (or already gone).
CREATE TABLE file_cleanup (
    key        TEXT    PRIMARY KEY,
    reason     TEXT    NOT NULL,
    attempts   INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
