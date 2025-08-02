import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { Room } from "@prisma/client";

class RoomRepository extends BaseRepository<Room> {
  async getAll(): Promise<Room[]> {
    return prisma.room.findMany();
  }

  async getById(id: number): Promise<Room | null> {
    return prisma.room.findUnique({ where: { id } });
  }

  async create(data: any): Promise<Room> {
    return prisma.room.create({ data });
  }

  async update(id: number, data: any): Promise<Room> {
    return prisma.room.update({ where: { id }, data });
  }

  async delete(id: number): Promise<Room> {
    return prisma.room.delete({ where: { id } });
  }
}

export default RoomRepository;
