import { prisma } from "../../src/lib/prisma";
import BillRepository from "../../src/repository/BillRepository";

jest.mock("../../src/lib/prisma", () => ({
  prisma: {
    bill: {
      findMany: jest.fn(),
      findUnique: jest.fn(),
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
    },
    tenant: {
      findUnique: jest.fn(),
    },
    electricityReading: {
      findUnique: jest.fn(),
    },
  },
}));

describe("BillRepository", () => {
  const billRepository = new BillRepository();

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe("getAll", () => {
    it("should return all bills", async () => {
      const mockBills = [{ id: 1, tenantId: 2 }];
      (prisma.bill.findMany as jest.Mock).mockResolvedValue(mockBills);

      const result = await billRepository.getAll();

      expect(prisma.bill.findMany).toHaveBeenCalled();
      expect(result).toEqual(mockBills);
    });
  });

  describe("getById", () => {
    it("should return a bill by id", async () => {
      const mockBill = { id: 1, tenantId: 2 };
      (prisma.bill.findUnique as jest.Mock).mockResolvedValue(mockBill);

      const result = await billRepository.getById(1);

      expect(prisma.bill.findUnique).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockBill);
    });

    it("should return null if bill not found", async () => {
      (prisma.bill.findUnique as jest.Mock).mockResolvedValue(null);

      const result = await billRepository.getById(999);

      expect(prisma.bill.findUnique).toHaveBeenCalledWith({ where: { id: 999 } });
      expect(result).toBeNull();
    });
  });

  describe("create", () => {
    it("should create a new bill successfully", async () => {
      const input = {
        tenantId: 2,
        readingId: 5,
        electricityRate: 10,
        additionalCharges: 20,
        additionalDescription: "Extra fee",
      };

      // Mock tenantWithRoom lookup
      (prisma.tenant.findUnique as jest.Mock).mockResolvedValue({
        room: { rent: 100 },
      });

      // Mock electricityReading lookup
      (prisma.electricityReading.findUnique as jest.Mock).mockResolvedValue({
        consumption: 15,
      });

      const createdBill = {
        id: 1,
        tenantId: 2,
        readingId: 5,
        roomCharges: 100,
        electricCharges: 150, // 15 * 10
        additionalCharges: 20,
        additionalDescription: "Extra fee",
        totalAmount: 270, // 100 + 150 + 20
      };

      (prisma.bill.create as jest.Mock).mockResolvedValue(createdBill);

      const result = await billRepository.create(input);

      expect(prisma.tenant.findUnique).toHaveBeenCalledWith({
        where: { id: input.tenantId },
        select: { room: { select: { rent: true } } },
      });

      expect(prisma.electricityReading.findUnique).toHaveBeenCalledWith({
        where: { id: input.readingId },
        select: { consumption: true },
      });

      expect(prisma.bill.create).toHaveBeenCalledWith({
        data: {
          tenantId: input.tenantId,
          readingId: input.readingId,
          roomCharges: 100,
          electricCharges: 150,
          additionalCharges: 20,
          additionalDescription: "Extra fee",
          totalAmount: 270,
        },
      });

      expect(result).toEqual(createdBill);
    });

    it("should throw error if electricity reading not found", async () => {
      const input = {
        tenantId: 2,
        readingId: 5,
        electricityRate: 10,
      };

      (prisma.tenant.findUnique as jest.Mock).mockResolvedValue({
        room: { rent: 100 },
      });
      (prisma.electricityReading.findUnique as jest.Mock).mockResolvedValue(null);

      await expect(billRepository.create(input)).rejects.toThrow("Electricity reading not found");
    });
  });

  describe("update", () => {
    it("should update a bill", async () => {
      const updatedBill = { id: 1, tenantId: 2, totalAmount: 200 };
      (prisma.bill.update as jest.Mock).mockResolvedValue(updatedBill);

      const data = { totalAmount: 200 };
      const result = await billRepository.update(1, data);

      expect(prisma.bill.update).toHaveBeenCalledWith({ where: { id: 1 }, data });
      expect(result).toEqual(updatedBill);
    });
  });

  describe("delete", () => {
    it("should delete a bill", async () => {
      const deletedBill = { id: 1, tenantId: 2 };
      (prisma.bill.delete as jest.Mock).mockResolvedValue(deletedBill);

      const result = await billRepository.delete(1);

      expect(prisma.bill.delete).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(deletedBill);
    });
  });
});
