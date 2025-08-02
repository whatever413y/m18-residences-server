import "express-async-errors";
import express, { Express } from "express";
import cors from "cors";
import dotenv from 'dotenv';
import { RequestHandler } from 'express-serve-static-core';
import { prisma } from "./lib/prisma";

import TenantRoute from "./routes/TenantRoute";
import RoomRoute from "./routes/RoomRoute";
import ElectricityReadingRoute from "./routes/ElectricityReadingRoute";
import BillRoute from "./routes/BillRoute";
import errorHandler from "./middleware/errorHandler";

dotenv.config();

const app: Express = express();
const PORT: number = Number(process.env.PORT) || 3001;

// Middleware
app.use(cors({
  origin: process.env.NODE_ENV === 'production'
    ? ["https://your-production-site.com"]
    : ["http://localhost:3000", "http://localhost:5173"],
  methods: ["GET", "POST", "PUT", "DELETE"],
  // credentials: true,
}));

app.use(express.urlencoded({ extended: true }) as RequestHandler);
app.use(express.json() as RequestHandler);

// Routes
app.use(TenantRoute);
app.use(RoomRoute);
app.use(ElectricityReadingRoute);
app.use(BillRoute);

app.get("/", (req, res) => {
  res.send("Rental Management API is up");
});

app.get("/health", (req, res) => {
  res.status(200).json({ status: "ok" });
});


app.use("*", (req, res) => {
  res.status(404).json({ message: "Route not found" });
});

app.use(errorHandler);

// Server
app.listen(PORT, () => {
  console.log(`App listening on port ${PORT} in ${process.env.NODE_ENV || 'development'} mode`);
});

// Graceful shutdown (not strictly needed with Prisma, but nice to have)
process.on('SIGINT', async () => {
  console.log('Gracefully shutting down...');
  await prisma.$disconnect();
  process.exit(0);
});

process.on('SIGTERM', async () => {
  console.log('Gracefully shutting down...');
  await prisma.$disconnect();
  process.exit(0);
});

