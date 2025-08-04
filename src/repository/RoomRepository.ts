import BaseRepository from "./BaseRepository";
import { prisma } from "../lib/prisma";
import { Room } from "@prisma/client";

type RoomData = Omit<Room, 'id' | 'createdAt' | 'updatedAt'>;

class RoomRepository extends BaseRepository<Room> {
  async getAll(): Promise<Room[]> {
    return prisma.room.findMany({ orderBy: { name: "asc" } });
  }

  async getById(id: number): Promise<Room | null> {
    return prisma.room.findUnique({ where: { id } });
  }

  async create(data: RoomData): Promise<Room> {
    return prisma.room.create({ data });
  }

  async update(id: number, data: RoomData): Promise<Room> {
    return prisma.room.update({ where: { id }, data });
  }

  async delete(id: number): Promise<Room> {
    return prisma.room.delete({ where: { id } });
  }
}

export default RoomRepository;
