-- Manual rename fix to resolve Prisma migration drift

ALTER TABLE "Bills" RENAME COLUMN "readingId" TO "reading_id";
ALTER TABLE "Tenants" RENAME COLUMN "isActive" TO "is_active";

-- Optional: rename index/constraints if necessary:
-- ALTER INDEX "Bills_readingId_key" RENAME TO "Bills_readingId_key";
-- ALTER TABLE "Bills" RENAME CONSTRAINT "Bills_readingId_fkey" TO "Bills_readingId_fkey";
