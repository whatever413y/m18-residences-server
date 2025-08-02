import BaseRepository from "../../src/repository/BaseRepository";
import { prisma } from "../../src/lib/prisma";

jest.mock("../../src/lib/prisma", () => ({
  prisma: {
    room: {
      findMany: jest.fn(),
      findUnique: jest.fn(),
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
    },
  },
}));

describe("BaseRepository with Prisma", () => {
  class TestRepository extends BaseRepository<any> {
    async getAll() {
      return prisma.room.findMany();
    }
    async getById(id: number) {
      return prisma.room.findUnique({ where: { id } });
    }
    async create(data: any) {
      return prisma.room.create({ data });
    }
    async update(id: number, data: any) {
      return prisma.room.update({ where: { id }, data });
    }
    async delete(id: number) {
      await prisma.room.delete({ where: { id } });
    }
  }

  const repo = new TestRepository();

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe("getAll", () => {
    it("should return all records", async () => {
      const mockRows = [{ id: 1, name: "Test" }];
      (prisma.room.findMany as jest.Mock).mockResolvedValue(mockRows);

      const result = await repo.getAll();

      expect(prisma.room.findMany).toHaveBeenCalled();
      expect(result).toEqual(mockRows);
    });
  });

  describe("getById", () => {
    it("should return a record by ID", async () => {
      const mockRow = { id: 1, name: "Test" };
      (prisma.room.findUnique as jest.Mock).mockResolvedValue(mockRow);

      const result = await repo.getById(1);

      expect(prisma.room.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockRow);
    });

    it("should return null if no record is found", async () => {
      (prisma.room.findUnique as jest.Mock).mockResolvedValue(null);

      const result = await repo.getById(1);

      expect(prisma.room.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toBeNull();
    });
  });

  describe("create", () => {
    it("should create a new record", async () => {
      const data = { name: "Test" };
      const mockRow = { id: 1, name: "Test" };
      (prisma.room.create as jest.Mock).mockResolvedValue(mockRow);

      const result = await repo.create(data);

      expect(prisma.room.create).toHaveBeenCalledWith({ data });
      expect(result).toEqual(mockRow);
    });
  });

  describe("update", () => {
    it("should update a record", async () => {
      const data = { name: "Updated Test" };
      const mockRow = { id: 1, name: "Updated Test" };
      (prisma.room.update as jest.Mock).mockResolvedValue(mockRow);

      const result = await repo.update(1, data);

      expect(prisma.room.update).toHaveBeenCalledWith({ where: { id: 1 }, data });
      expect(result).toEqual(mockRow);
    });
  });

  describe("delete", () => {
    it("should delete a record", async () => {
      (prisma.room.delete as jest.Mock).mockResolvedValue({});

      await repo.delete(1);

      expect(prisma.room.delete).toHaveBeenCalledWith({ where: { id: 1 } });
    });
  });
});
