import { prisma } from "../../src/lib/prisma";
import ElectricityReadingRepository from "../../src/repository/ElectricityReadingRepository";

jest.mock("../../src/lib/prisma", () => ({
  prisma: {
    electricityReading: {
      findMany: jest.fn(),
      findUnique: jest.fn(),
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
    },
  },
}));

describe("ElectricityReadingRepository", () => {
  const electricityReadingRepository = new ElectricityReadingRepository();

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe("getAll", () => {
    it("should return all records", async () => {
      const mockRecords = [{ id: 1, tenantId: 2 }];
      (prisma.electricityReading.findMany as jest.Mock).mockResolvedValue(mockRecords);

      const result = await electricityReadingRepository.getAll();

      expect(prisma.electricityReading.findMany).toHaveBeenCalled();
      expect(result).toEqual(mockRecords);
    });
  });

  describe("getById", () => {
    it("should return a record by ID", async () => {
      const mockRecord = { id: 1, tenantId: 2 };
      (prisma.electricityReading.findUnique as jest.Mock).mockResolvedValue(mockRecord);

      const result = await electricityReadingRepository.getById(1);

      expect(prisma.electricityReading.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockRecord);
    });

    it("should return null if no record is found", async () => {
      (prisma.electricityReading.findUnique as jest.Mock).mockResolvedValue(null);

      const result = await electricityReadingRepository.getById(1);

      expect(prisma.electricityReading.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toBeNull();
    });
  });

  describe("create", () => {
    it("should create a new record", async () => {
      const inputData = { tenantId: 2, roomId: 3, prevReading: 100, currReading: 150, consumption: 50 };
      const mockRecord = { id: 1, ...inputData };
      (prisma.electricityReading.create as jest.Mock).mockResolvedValue(mockRecord);

      const result = await electricityReadingRepository.create(inputData);

      expect(prisma.electricityReading.create).toHaveBeenCalledWith({ data: inputData });
      expect(result).toEqual(mockRecord);
    });
  });

  describe("update", () => {
    it("should update a record", async () => {
      const updateData = { currReading: 200, consumption: 100 };
      const mockRecord = { id: 1, tenantId: 2, ...updateData };
      (prisma.electricityReading.update as jest.Mock).mockResolvedValue(mockRecord);

      const result = await electricityReadingRepository.update(1, updateData);

      expect(prisma.electricityReading.update).toHaveBeenCalledWith({ where: { id: 1 }, data: updateData });
      expect(result).toEqual(mockRecord);
    });
  });

  describe("delete", () => {
    it("should delete a record", async () => {
      const mockRecord = { id: 1, tenantId: 2 };
      (prisma.electricityReading.delete as jest.Mock).mockResolvedValue(mockRecord);

      const result = await electricityReadingRepository.delete(1);

      expect(prisma.electricityReading.delete).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockRecord);
    });
  });
});
