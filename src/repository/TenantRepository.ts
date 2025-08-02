import { Tenant } from "@prisma/client";
import { prisma } from "../lib/prisma";
import BaseRepository from "./BaseRepository";

type TenantWithRoomName = {
  id: number;
  name: string;
  createdAt: Date;
  updatedAt: Date;
  roomId: number;
  roomName?: string;
};
class TenantRepository extends BaseRepository<Tenant> {
  async getAll(): Promise<TenantWithRoomName[]> {
    const tenants = await prisma.tenant.findMany({
      select: {
        id: true,
        name: true,
        createdAt: true,
        updatedAt: true,
        roomId: true,       
        room: {
          select: {
            name: true,
          },
        },
      },
    });
    return tenants.map(t => ({
      id: t.id,
      name: t.name,
      createdAt: t.createdAt,
      updatedAt: t.updatedAt,
      roomId: t.roomId,
      roomName: t.room?.name,
    }));
  }

  async getById(id: number): Promise<Tenant | null> {
    return prisma.tenant.findUnique({ where: { id } });
  }

  async create(data: any): Promise<Tenant> {
    return prisma.tenant.create({ data });
  }

  async update(id: number, data: any): Promise<Tenant> {
    return prisma.tenant.update({ where: { id }, data });
  }

  async delete(id: number): Promise<Tenant> {
    return prisma.tenant.delete({ where: { id } });
  }
}

export default TenantRepository;
