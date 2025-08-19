import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { AdditionalCharge, Bill } from "@prisma/client";

type AdditionalChargeInput = {
  amount: number;
  description: string;
};

type BillWithChargesAndReading = {
  bill: Bill;
  additional_charges: AdditionalCharge[];
  reading: {
    prev_reading: number;
    curr_reading: number;
    consumption: number;
  } | null;
};

type BillData = Omit<
  Bill,
  "id" | "total_amount" | "created_at" | "updated_at"
> & {
  additional_charges: AdditionalChargeInput[];
  receipt_url: string;
};

class BillRepository {
  private wrapBill(bill: Bill | null): BillWithChargesAndReading | null {
    if (!bill) return null;
    const { additional_charges, reading, ...billData } = bill as any;
    return {
      bill: billData,
      reading: reading ?? null,
      additional_charges: additional_charges ?? [],
    };
  }

  async getAll(): Promise<BillWithChargesAndReading[]> {
    const bills = await prisma.bill.findMany({
      orderBy: { created_at: "desc" },
      include: {
        additional_charges: true,
        reading: {
          select: {
            prev_reading: true,
            curr_reading: true,
            consumption: true,
          },
        },
      },
    });
    return bills.map((bill) => this.wrapBill(bill)!);
  }

  async getAllById(tenantId: number): Promise<BillWithChargesAndReading[]> {
    const bills = await prisma.bill.findMany({
      where: { tenant_id: tenantId },
      orderBy: { created_at: "desc" },
      include: {
        additional_charges: true,
        reading: {
          select: {
            prev_reading: true,
            curr_reading: true,
            consumption: true,
          },
        },
      },
    });
    return bills.map((bill) => this.wrapBill(bill)!);
  }

  async getById(id: number): Promise<BillWithChargesAndReading | null> {
    const bill = await prisma.bill.findFirst({
      where: { id },
      orderBy: { created_at: "desc" },
      include: {
        additional_charges: true,
        reading: {
          select: {
            prev_reading: true,
            curr_reading: true,
            consumption: true,
          },
        },
      },
    });
    return this.wrapBill(bill);
  }

  async getByTenantId(id: number): Promise<BillWithChargesAndReading | null> {
    const bill = await prisma.bill.findFirst({
      where: { tenant_id: id },
      orderBy: { created_at: "desc" },
      include: {
        additional_charges: true,
        reading: {
          select: {
            prev_reading: true,
            curr_reading: true,
            consumption: true,
          },
        },
      },
    });
    return this.wrapBill(bill);
  }

  async create(data: BillData): Promise<BillWithChargesAndReading> {
    const { additional_charges, ...billData } = data;

    const additionalChargesTotal = additional_charges.reduce(
      (acc, curr) => acc + curr.amount,
      0
    );

    const total_amount =
      (billData.room_charges ?? 0) +
      (billData.electric_charges ?? 0) +
      additionalChargesTotal;

    return prisma.$transaction(async (tx) => {
      const bill = await tx.bill.create({
        data: {
          ...billData,
          total_amount,
        },
      });

      for (const charge of additional_charges) {
        await tx.additionalCharge.create({
          data: {
            bill_id: bill.id,
            amount: charge.amount,
            description: charge.description,
          },
        });
      }

      return this.wrapBill(bill)!;
    });
  }

  async update(id: number, data: BillData): Promise<BillWithChargesAndReading> {
    const { additional_charges, receipt_url, ...billData } = data;

    const additionalChargesTotal = additional_charges.reduce(
      (acc, curr) => Number(acc) + Number(curr.amount),
      0
    );

    const total_amount =
      (billData.room_charges ?? 0) +
      (billData.electric_charges ?? 0) +
      additionalChargesTotal;

    return prisma.$transaction(async (tx) => {
      let paid = false;
      if (receipt_url != null) {
        paid = true;
      }

      const bill = await tx.bill.update({
        where: { id },
        data: {
          ...billData,
          total_amount,
          paid,
          receipt_url,
        },
      });

      await tx.additionalCharge.deleteMany({ where: { bill_id: id } });

      for (const charge of additional_charges) {
        await tx.additionalCharge.create({
          data: {
            bill_id: id,
            amount: Number(charge.amount),
            description: charge.description,
          },
        });
      }

      return this.wrapBill(bill)!;
    });
  }

  async delete(id: number): Promise<Bill> {
    return prisma.bill.delete({ where: { id } });
  }
}

export default BillRepository;
