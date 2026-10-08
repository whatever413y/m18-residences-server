-- Indexes for the bill list's filters (recent bills, a tenant's bills by date, open bills without a receipt) and for
-- readings by room.
CREATE INDEX bill_created_at_idx ON bill (created_at);
CREATE INDEX bill_tenant_created_at_idx ON bill (tenant_id, created_at);
CREATE INDEX bill_receipt_url_idx ON bill (receipt_url);
CREATE INDEX electricity_reading_room_id_idx ON electricity_reading (room_id);
