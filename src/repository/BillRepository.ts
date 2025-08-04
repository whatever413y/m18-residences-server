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

  async create(data: any): Promise<Bill> {
    return prisma.bill.create({ data });
  }

  async update(id: number, data: any): Promise<Bill> {
    return prisma.bill.update({ where: { id }, data });
  }

  async delete(id: number): Promise<Bill> {
    return prisma.bill.delete({ where: { id } });
  }
}

export default BillRepository;
