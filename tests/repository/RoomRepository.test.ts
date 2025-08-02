import { prisma } from "../../src/lib/prisma";
import RoomRepository from "../../src/repository/RoomRepository";

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

describe("RoomRepository (Prisma)", () => {
  const roomRepository = new RoomRepository();

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe("getAll", () => {
    it("should return all rooms", async () => {
      const mockRooms = [{ id: 1, name: "Room 101" }];
      (prisma.room.findMany as jest.Mock).mockResolvedValue(mockRooms);

      const result = await roomRepository.getAll();

      expect(prisma.room.findMany).toHaveBeenCalled();
      expect(result).toEqual(mockRooms);
    });
  });

  describe("getById", () => {
    it("should return a room by ID", async () => {
      const mockRoom = { id: 1, name: "Room 101" };
      (prisma.room.findUnique as jest.Mock).mockResolvedValue(mockRoom);

      const result = await roomRepository.getById(1);

      expect(prisma.room.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockRoom);
    });

    it("should return null if room not found", async () => {
      (prisma.room.findUnique as jest.Mock).mockResolvedValue(null);

      const result = await roomRepository.getById(999);

      expect(prisma.room.findUnique).toHaveBeenCalledWith({
        where: { id: 999 },
      });
      expect(result).toBeNull();
    });
  });

  describe("create", () => {
    it("should create a new room", async () => {
      const mockRoom = { id: 1, name: "New Room" };
      (prisma.room.create as jest.Mock).mockResolvedValue(mockRoom);

      const result = await roomRepository.create({ name: "New Room" });

      expect(prisma.room.create).toHaveBeenCalledWith({
        data: { name: "New Room" },
      });
      expect(result).toEqual(mockRoom);
    });
  });

  describe("update", () => {
    it("should update a room", async () => {
      const mockRoom = { id: 1, name: "Updated Room" };
      (prisma.room.update as jest.Mock).mockResolvedValue(mockRoom);

      const result = await roomRepository.update(1, { name: "Updated Room" });

      expect(prisma.room.update).toHaveBeenCalledWith({
        where: { id: 1 },
        data: { name: "Updated Room" },
      });

      expect(result).toEqual(mockRoom);
    });
  });

  describe("delete", () => {
    it("should delete a room", async () => {
      const mockRoom = { id: 1, name: "Deleted Room" };
      (prisma.room.delete as jest.Mock).mockResolvedValue(mockRoom);

      const result = await roomRepository.delete(1);

      expect(prisma.room.delete).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockRoom);
    });
  });
});
