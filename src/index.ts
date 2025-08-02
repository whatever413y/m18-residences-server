import express, { Express } from "express";
import cors from "cors";
import dotenv from 'dotenv';
import { RequestHandler } from 'express-serve-static-core';
import db from "./config/Database"

import TenantRoute from "./routes/TenantRoute"
import RoomRoute from "./routes/RoomRoute"
import ElectricityReadingRoute from "./routes/ElectricityReadingRoute"
import BillRoute from "./routes/BillRoute";
import errorHandler from "./middleware/errorHandler";

dotenv.config()

const app: Express = express();
const PORT: number = Number(process.env.PORT);

app.use(cors({
  origin: ["http://localhost:3000", "http://localhost:5173"],
  methods: ["GET", "POST", "PUT", "DELETE"],
  // credentials: true, // Add this if you use cookies or sessions
}));

app.use(express.urlencoded({ extended: true }) as RequestHandler);
app.use(express.json() as RequestHandler);

db.connect()
  .then(() => console.log("Database connected successfully."))
  .catch((err) => {
    console.error("Database connection error:", err);
    process.exit(1);
  });

app.use(TenantRoute);
app.use(RoomRoute);
app.use(ElectricityReadingRoute);
app.use(BillRoute);

app.get("/", (req, res) => {
  res.send("Rental Management API is up");
});

app.use("*", (req, res) => {
  res.status(404).json({ message: "Route not found" });
});

app.use(errorHandler);

app.listen(PORT, () => {
  console.log(`App listening on port ${PORT}`);
});

process.on('SIGINT', async () => {
  console.log('Gracefully shutting down...');
  await db.end();
  process.exit(0);
});