import { prisma } from "../src/lib/prisma";

async function main() {
  await prisma.room.createMany({
    data: [
      { name: "Blue", rent: 10000 },
      { name: "Green", rent: 8500 },
      { name: "Orange", rent: 3500 },
      { name: "Pink", rent: 5500 },
      { name: "Violet", rent: 6500 },
      { name: "Yellow", rent: 6500 },
    ],
    skipDuplicates: true,
  });
}

main()
  .then(() => {
    console.log("Seeding completed.");
  })
  .catch((e) => {
    console.error(e);
    process.exit(1);
  })
  .finally(async () => {
    await prisma.$disconnect();
  });
