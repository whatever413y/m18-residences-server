import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { Bill } from "@prisma/client";

type AdditionalChargeInput = {
  amount: number;
  description: string;
};

type BillData = Omit<
  Bill,
  | "id"
  | "totalAmount"
  | "createdAt"
  | "updatedAt"
> & {
  additionalCharges: AdditionalChargeInput[];
};

class BillRepository extends BaseRepository<Bill, BillData> {
  async getAll(): Promise<Bill[]> {
    return prisma.bill.findMany({ orderBy: { createdAt: "desc" }, include: {
      additionalCharges: true,
    }, });
  }

  async getAllById(tenantId: number): Promise<Bill[]> {
    return prisma.bill.findMany({
      where: { tenantId },
      orderBy: { createdAt: "desc" },
      include: {
        additionalCharges: true,
        reading: {
          select: {
            prevReading: true,
            currReading: true,
            consumption: true,
          },
        },
      },
    });
  }

  async getById(id: number): Promise<Bill | null> {
    return prisma.bill.findFirst({
      where: { tenantId: id },
      orderBy: { createdAt: "desc" },
      include: {
        additionalCharges: true,
        reading: {
          select: {
            prevReading: true,
            currReading: true,
            consumption: true,
          },
        },
      },
    });
  }
  
  async create(data: BillData): Promise<Bill> {
    const { additionalCharges, ...billData } = data;

    const additionalChargesTotal = additionalCharges.reduce(
      (acc, curr) => acc + curr.amount,
      0
    );

    const totalAmount =
      (billData.roomCharges ?? 0) +
      (billData.electricCharges ?? 0) +
      additionalChargesTotal;

    return prisma.$transaction(async (tx) => {
      const bill = await tx.bill.create({
        data: {
          ...billData,
          totalAmount,
        },
      });

      for (const charge of additionalCharges) {
        await tx.additionalCharge.create({
          data: {
            billId: bill.id,
            amount: charge.amount,
            description: charge.description,
          },
        });
      }

      return bill;
    });
  }

  async update(
    id: number,
    data: BillData,
  ): Promise<Bill> {
    const { additionalCharges: _, ...billData } = data;

    const additionalChargesTotal = data.additionalCharges.reduce(
      (acc, curr) => acc + curr.amount,
      0
    );

    const totalAmount =
      (billData.roomCharges ?? 0) +
      (billData.electricCharges ?? 0) +
      additionalChargesTotal;

    return prisma.$transaction(async (tx) => {
      const bill = await tx.bill.update({
        where: { id },
        data: {
          ...billData,
          totalAmount,
        },
      });

      await tx.additionalCharge.deleteMany({ where: { billId: id } });

      for (const charge of data.additionalCharges) {
        await tx.additionalCharge.create({
          data: {
            billId: id,
            amount: charge.amount,
            description: charge.description,
          },
        });
      }

      return bill;
    });
  }

  async delete(id: number): Promise<Bill> {
    return prisma.bill.delete({ where: { id } });
  }
}

export default BillRepository;
