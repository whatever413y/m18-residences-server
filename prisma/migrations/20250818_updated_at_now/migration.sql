ALTER TABLE "Rooms" ALTER COLUMN "updated_at" SET DEFAULT NOW();
ALTER TABLE "Tenants" ALTER COLUMN "updated_at" SET DEFAULT NOW();
ALTER TABLE "Electricity_Readings" ALTER COLUMN "updated_at" SET DEFAULT NOW();
ALTER TABLE "Bills" ALTER COLUMN "updated_at" SET DEFAULT NOW();
ALTER TABLE "Additional_Charges" ALTER COLUMN "updated_at" SET DEFAULT NOW();
