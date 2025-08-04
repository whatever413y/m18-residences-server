import { PrismaClient } from '@prisma/client';

// Create a single PrismaClient instance
const prismaBase = new PrismaClient({
  log: ['query'],
});

// Use global to hold PrismaClient instance to prevent multiple instances in dev
const globalForPrisma = globalThis as unknown as {
  prisma?: PrismaClient;
};

export const prisma =
  process.env.NODE_ENV === 'production'
    ? prismaBase
    : globalForPrisma.prisma || (() => {
        globalForPrisma.prisma = prismaBase;
        return prismaBase;
      })();
