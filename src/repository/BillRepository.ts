import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { Bill } from "@prisma/client";

type BillData = Omit<Bill, "id" | "totalAmount" | "createdAt" | "updatedAt">;

class BillRepository extends BaseRepository<Bill> {
  async getAll(): Promise<Bill[]> {
    return prisma.bill.findMany({ orderBy: { createdAt: "desc" } });
  }

  async getById(id: number): Promise<Bill | null> {
    return prisma.bill.findUnique({ where: { id } });
  }

  async create(data: BillData): Promise<Bill> {
    const totalAmount =
      (data.roomCharges ?? 0) +
      (data.electricCharges ?? 0) +
      (data.additionalCharges ?? 0);

    return prisma.bill.create({
      data: {
        ...data,
        totalAmount,
      },
    });
  }

  async update(id: number, data: BillData): Promise<Bill> {
    const totalAmount =
      (data.roomCharges ?? 0) +
      (data.electricCharges ?? 0) +
      (data.additionalCharges ?? 0);

    return prisma.bill.update({
      where: { id },
      data: {
        ...data,
        totalAmount,
      },
    });
  }

  async delete(id: number): Promise<Bill> {
    return prisma.bill.delete({ where: { id } });
  }
}

export default BillRepository;
