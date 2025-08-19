import { AdditionalCharge } from "@prisma/client";
import { prisma } from "../lib/prisma";
import BaseRepository from "./BaseRepository";

type AdditionalChargeData = Omit<AdditionalCharge, "id" | "createdAt" | "updatedAt">;

class AdditionalChargeRepository extends BaseRepository<AdditionalCharge> {
    async getAll(): Promise<AdditionalCharge[]> {
        return prisma.additionalCharge.findMany({
            orderBy: { created_at: "asc" },
        });
    }

    async getAllById(billId: number): Promise<AdditionalCharge[]> {
        return prisma.additionalCharge.findMany({
            where: { bill_id: billId },
            orderBy: { created_at: "asc" },
        });
  }

  async getById(id: number): Promise<AdditionalCharge | null> {
    return prisma.additionalCharge.findUnique({ where: { id } });
  }

  async create(data: AdditionalChargeData): Promise<AdditionalCharge> {
    return prisma.additionalCharge.create({ data });
  }

  async update(id: number, data: Partial<AdditionalChargeData>): Promise<AdditionalCharge> {
    return prisma.additionalCharge.update({ where: { id }, data });
  }

  async delete(id: number): Promise<AdditionalCharge> {
    return prisma.additionalCharge.delete({ where: { id } });
  }

  async deleteManyByBillId(billId: number): Promise<void> {
    await prisma.additionalCharge.deleteMany({ where: { bill_id: billId } });
  }
}

export default AdditionalChargeRepository;
