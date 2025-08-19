import { ElectricityReading } from "@prisma/client";
import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";

type ElectricityReadingData = Omit<
  ElectricityReading,
  "id" | "consumption" | "createdAt" | "updatedAt"
>;

class ElectricityReadingRepository extends BaseRepository<ElectricityReading> {
  async getAll(): Promise<ElectricityReading[]> {
    return prisma.electricityReading.findMany({
      orderBy: { created_at: "desc" },
    });
  }

  async getById(id: number): Promise<ElectricityReading | null> {
    return prisma.electricityReading.findUnique({ where: { id } });
  }

  async create(data: ElectricityReadingData): Promise<ElectricityReading> {
    const consumption = data.curr_reading - data.prev_reading;
    return prisma.electricityReading.create({
      data: {
        ...data,
        consumption: consumption,
      },
    });
  }

  async update(
    id: number,
    data: ElectricityReadingData
  ): Promise<ElectricityReading> {
    const consumption = data.curr_reading - data.prev_reading;
    return prisma.electricityReading.update({
      where: { id },
      data: {
        ...data,
        consumption: consumption,
      },
    });
  }

  async delete(id: number): Promise<ElectricityReading> {
    return prisma.electricityReading.delete({ where: { id } });
  }
}

export default ElectricityReadingRepository;
