import { prisma } from "../src/lib/prisma";

async function main() {
  await prisma.room.createMany({
    data: [
      { name: "Blue", rent: 1500 },
      { name: "Green", rent: 1300 },
      { name: "Orange", rent: 1400 },
      { name: "Pink", rent: 1200 },
      { name: "Violet", rent: 1100 },
      { name: "Yellow", rent: 1000 },
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
