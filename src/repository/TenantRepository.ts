import { Tenant } from "@prisma/client";
import { prisma } from "../lib/prisma";
import BaseRepository from "./BaseRepository";

type TenantData = Omit<Tenant, "id" | "createdAt" | "updatedAt">;

class TenantRepository extends BaseRepository<Tenant> {
  async getAll(): Promise<Tenant[]> {
    return prisma.tenant.findMany({
      orderBy: {
        joinDate: "desc",
      },
    });
  }

  async getById(id: number): Promise<Tenant | null> {
    return prisma.tenant.findUnique({ where: { id } });
  }

  async getByTenantName(tenantName: string): Promise<Tenant | null> {
    return prisma.tenant.findFirst({
      where: { name: tenantName },
    });
  }

  async create(data: TenantData): Promise<Tenant> {
    return prisma.tenant.create({
      data: {
        name: data.name,
        roomId: data.roomId,
        joinDate: new Date(data.joinDate),
      },
    });
  }

  async update(
    id: number,
    data: TenantData
  ): Promise<Tenant> {
    return prisma.tenant.update({
      where: { id },
      data: {
        name: data.name,
        roomId: data.roomId,
        joinDate: new Date(data.joinDate),
      },
    });
  }

  async delete(id: number): Promise<Tenant> {
    return prisma.tenant.delete({ where: { id } });
  }
}

export default TenantRepository;
