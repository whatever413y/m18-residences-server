import { PrismaClient } from '@prisma/client';

const globalForPrisma = globalThis as unknown as { prisma: PrismaClient };

const prismaInstance = globalForPrisma.prisma || new PrismaClient({
  log: ['query'],
});

// ✅ Middleware to auto-compute `consumption` for ElectricityReading
prismaInstance.$use(async (params, next) => {
  if (
    params.model === 'ElectricityReading' &&
    (params.action === 'create' || params.action === 'update')
  ) {
    const data = params.args.data;

    const prev = data.prevReading;
    const curr = data.currReading;

    if (typeof prev === 'number' && typeof curr === 'number') {
      data.consumption = curr - prev;
    }
  }

  return next(params);
});

// ✅ Middleware to auto-calculate totalAmount in Bill
prismaInstance.$use(async (params, next) => {
  if (
    params.model === 'Bill' &&
    (params.action === 'create' || params.action === 'update')
  ) {
    const data = params.args.data;

    const roomCharges = data.roomCharges ?? 0;
    const electricCharges = data.electricCharges ?? 0;
    const additionalCharges = data.additionalCharges ?? 0;

    data.totalAmount = roomCharges + electricCharges + additionalCharges;
  }

  return next(params);
});

export const prisma = prismaInstance;

if (process.env.NODE_ENV !== 'production') {
  globalForPrisma.prisma = prisma;
}
