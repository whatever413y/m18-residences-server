-- CreateTable
CREATE TABLE "public"."Rooms" (
    "id" SERIAL NOT NULL,
    "name" TEXT NOT NULL,
    "rent" INTEGER NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Rooms_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "public"."Tenants" (
    "id" SERIAL NOT NULL,
    "room_id" INTEGER NOT NULL,
    "name" TEXT NOT NULL,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    "join_date" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Tenants_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "public"."Electricity_Readings" (
    "id" SERIAL NOT NULL,
    "tenant_id" INTEGER NOT NULL,
    "room_id" INTEGER NOT NULL,
    "prev_reading" INTEGER NOT NULL,
    "curr_reading" INTEGER NOT NULL,
    "consumption" INTEGER NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Electricity_Readings_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "public"."Bills" (
    "id" SERIAL NOT NULL,
    "readingId" INTEGER NOT NULL,
    "tenant_id" INTEGER NOT NULL,
    "room_charges" INTEGER NOT NULL DEFAULT 0,
    "electric_charges" INTEGER NOT NULL DEFAULT 0,
    "total_amount" INTEGER NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Bills_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "public"."Additional_Charges" (
    "id" SERIAL NOT NULL,
    "bill_id" INTEGER NOT NULL,
    "amount" INTEGER NOT NULL DEFAULT 0,
    "description" TEXT NOT NULL,
    "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "Additional_Charges_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "Rooms_name_key" ON "public"."Rooms"("name");

-- CreateIndex
CREATE UNIQUE INDEX "Tenants_name_key" ON "public"."Tenants"("name");

-- CreateIndex
CREATE UNIQUE INDEX "Bills_readingId_key" ON "public"."Bills"("readingId");

-- AddForeignKey
ALTER TABLE "public"."Tenants" ADD CONSTRAINT "Tenants_room_id_fkey" FOREIGN KEY ("room_id") REFERENCES "public"."Rooms"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "public"."Electricity_Readings" ADD CONSTRAINT "Electricity_Readings_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "public"."Tenants"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "public"."Electricity_Readings" ADD CONSTRAINT "Electricity_Readings_room_id_fkey" FOREIGN KEY ("room_id") REFERENCES "public"."Rooms"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "public"."Bills" ADD CONSTRAINT "Bills_readingId_fkey" FOREIGN KEY ("readingId") REFERENCES "public"."Electricity_Readings"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "public"."Bills" ADD CONSTRAINT "Bills_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "public"."Tenants"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "public"."Additional_Charges" ADD CONSTRAINT "Additional_Charges_bill_id_fkey" FOREIGN KEY ("bill_id") REFERENCES "public"."Bills"("id") ON DELETE CASCADE ON UPDATE CASCADE;
