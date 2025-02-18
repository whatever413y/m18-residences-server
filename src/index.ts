import express, { Express } from "express";
import cors from "cors";
import dotenv from 'dotenv';
import { RequestHandler } from 'express-serve-static-core';
import db from "./config/Database"

import TenantRoute from "./routes/TenantRoute"
import RoomRoute from "./routes/RoomRoute"
import ElectricityReadingRoute from "./routes/ElectricityReadingRoute"
import BillRoute from "./routes/BillRoute";
import { userInfo } from "os";

dotenv.config()

const app: Express = express();
const PORT: number = 3001;

app.use(cors());
app.use(express.urlencoded({extended: true}) as RequestHandler)
app.use(express.json() as RequestHandler);

db.connect((err) => {
  if (err) {
    console.log(err)
  } else {
    console.log("DB Connected!")
  }
})

app.use(TenantRoute)
app.use(RoomRoute)
app.use(ElectricityReadingRoute)
app.use(BillRoute)

app.listen(PORT, () => {
  console.log(`App listening on port ${PORT}`);
});