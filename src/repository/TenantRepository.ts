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
  joinDate: Date;
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
        joinDate: true,
      },
    });
    return tenants.map(t => ({
      id: t.id,
      name: t.name,
      createdAt: t.createdAt,
      updatedAt: t.updatedAt,
      roomId: t.roomId,
      roomName: t.room?.name,
      joinDate: t.joinDate,
    }));
  }

  async getById(id: number): Promise<Tenant | null> {
    return prisma.tenant.findUnique({ where: { id } });
  }

  async create(data: { name: string; roomId: number, joinDate: string}): Promise<Tenant> {
  return prisma.tenant.create({
    data: {
      name: data.name,
      roomId: data.roomId,
      joinDate: new Date(data.joinDate),
    },
  });
}


  async update(id: number, data: { name: string; roomId: number, joinDate: string}): Promise<Tenant> {
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
