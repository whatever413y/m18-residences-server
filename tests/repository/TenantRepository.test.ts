import { prisma } from "../../src/lib/prisma";
import TenantRepository from "../../src/repository/TenantRepository";

jest.mock("../../src/lib/prisma", () => ({
  prisma: {
    tenant: {
      findMany: jest.fn(),
      findUnique: jest.fn(),
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
    },
  },
}));

describe("TenantRepository (Prisma)", () => {
  const tenantRepository = new TenantRepository();

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe("getAll", () => {
    it("should return all tenants with room info", async () => {
      const mockTenants = [
        {
          id: 1,
          name: "Test Tenant",
          createdAt: "2025-08-01T00:00:00.000Z",
          updatedAt: "2025-08-01T00:00:00.000Z",
          roomId: 101,
          room: { name: "Room A" },
          joinDate: new Date("2025-08-01"),
        },
      ];
      (prisma.tenant.findMany as jest.Mock).mockResolvedValue(mockTenants);

      const result = await tenantRepository.getAll();

      expect(prisma.tenant.findMany).toHaveBeenCalledWith({
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

      expect(result).toEqual([
        {
          id: 1,
          name: "Test Tenant",
          createdAt: "2025-08-01T00:00:00.000Z",
          updatedAt: "2025-08-01T00:00:00.000Z",
          roomId: 101,
          roomName: "Room A",
          joinDate: new Date("2025-08-01"),
        },
      ]);
    });
  });

  describe("getById", () => {
    it("should return a tenant by ID", async () => {
      const mockTenant = { id: 1, name: "Test Tenant" };
      (prisma.tenant.findUnique as jest.Mock).mockResolvedValue(mockTenant);

      const result = await tenantRepository.getById(1);

      expect(prisma.tenant.findUnique).toHaveBeenCalledWith({
        where: { id: 1 },
      });
      expect(result).toEqual(mockTenant);
    });

    it("should return null if tenant not found", async () => {
      (prisma.tenant.findUnique as jest.Mock).mockResolvedValue(null);

      const result = await tenantRepository.getById(999);

      expect(prisma.tenant.findUnique).toHaveBeenCalledWith({
        where: { id: 999 },
      });
      expect(result).toBeNull();
    });
  });

  describe("create", () => {
    it("should create a new tenant", async () => {
      const mockTenant = { id: 1, name: "New Tenant" };
      (prisma.tenant.create as jest.Mock).mockResolvedValue(mockTenant);

      const result = await tenantRepository.create({
        name: "New Tenant",
        roomId: 0,
        joinDate: new Date("2024-06-01").toISOString(),
      });

      expect(prisma.tenant.create).toHaveBeenCalledWith({
        data: { name: "New Tenant", roomId: 0, joinDate: expect.any(Date) },
      });
      expect(result).toEqual(mockTenant);
    });
  });

  describe("update", () => {
    it("should update a tenant", async () => {
      const mockTenant = { id: 1, name: "Updated Tenant" };
      (prisma.tenant.update as jest.Mock).mockResolvedValue(mockTenant);

      const result = await tenantRepository.update(1, {
        name: "Updated Tenant",
        roomId: 0,
        joinDate: new Date("2024-06-01").toISOString(),
      });

      expect(prisma.tenant.update).toHaveBeenCalledWith({
        where: { id: 1 },
        data: { name: "Updated Tenant", roomId: 0, joinDate: new Date("2024-06-01") },
      });
      expect(result).toEqual(mockTenant);
    });
  });

  describe("delete", () => {
    it("should delete a tenant", async () => {
      const mockTenant = { id: 1, name: "Deleted Tenant" };
      (prisma.tenant.delete as jest.Mock).mockResolvedValue(mockTenant);

      const result = await tenantRepository.delete(1);

      expect(prisma.tenant.delete).toHaveBeenCalledWith({ where: { id: 1 } });
      expect(result).toEqual(mockTenant);
    });
  });
});
