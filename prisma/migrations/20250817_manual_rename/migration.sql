-- Manual rename fix to resolve Prisma migration drift

ALTER TABLE "Bills" RENAME COLUMN "readingId" TO "reading_id";
ALTER TABLE "Tenants" RENAME COLUMN "isActive" TO "is_active";
