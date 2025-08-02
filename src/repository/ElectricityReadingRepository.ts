import { ElectricityReading } from "@prisma/client";
import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";

class ElectricityReadingRepository extends BaseRepository<ElectricityReading> {
  async getAll(): Promise<ElectricityReading[]> {
      return prisma.electricityReading.findMany();
    }
  
    async getById(id: number): Promise<ElectricityReading | null> {
      return prisma.electricityReading.findUnique({ where: { id } });
    }
  
    async create(data: any): Promise<ElectricityReading> {
      return prisma.electricityReading.create({ data });
    }
  
    async update(id: number, data: any): Promise<ElectricityReading> {
      return prisma.electricityReading.update({ where: { id }, data });
    }
  
    async delete(id: number): Promise<ElectricityReading> {
      return prisma.electricityReading.delete({ where: { id } });
    }

}

export default ElectricityReadingRepository;
