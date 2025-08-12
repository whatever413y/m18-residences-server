-- AlterTable
ALTER TABLE "public"."Bills" ADD COLUMN     "paid" BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN     "receipt_url" TEXT;
