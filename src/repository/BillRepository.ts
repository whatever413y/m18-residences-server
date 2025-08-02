import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { Bill } from "@prisma/client";

class BillRepository extends BaseRepository<Bill> {
  async getAll(): Promise<Bill[]> {
    return prisma.bill.findMany();
  }

  async getById(id: number): Promise<Bill | null> {
    return prisma.bill.findUnique({ where: { id } });
  }

  async create(input: {
    tenantId: number;
    readingId: number;
    electricityRate: number;
    additionalCharges?: number;
    additionalDescription?: string | null;
  }): Promise<Bill> {
    const {
      tenantId,
      readingId,
      electricityRate,
      additionalCharges = 0,
      additionalDescription = null,
    } = input;
    
    const tenantWithRoom = await prisma.tenant.findUnique({
      where: { id: tenantId },
      select: { room: { select: { rent: true } } },
    });

    const roomCharges = tenantWithRoom?.room?.rent ?? 0;

    const reading = await prisma.electricityReading.findUnique({
      where: { id: readingId },
      select: { consumption: true },
    });

    if (!reading) {
      throw new Error("Electricity reading not found");
    }

    const electricCharges = reading.consumption * electricityRate;

    const totalAmount = roomCharges + electricCharges + additionalCharges;

    return prisma.bill.create({
      data: {
        tenantId,
        readingId,
        roomCharges,
        electricCharges,
        additionalCharges,
        additionalDescription,
        totalAmount,
      },
    });
  }

  async update(id: number, data: any): Promise<Bill> {
    return prisma.bill.update({ where: { id }, data });
  }

  async delete(id: number): Promise<Bill> {
    return prisma.bill.delete({ where: { id } });
  }
}

export default BillRepository;
