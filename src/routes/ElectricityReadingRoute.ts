import ElectricityReadingController from "../controllers/ElectricityReadingController";
import BaseRoute from "./BaseRoute";

const ElectricityReadingRoute = new BaseRoute(new ElectricityReadingController(), "/electricity-readings");
const router = ElectricityReadingRoute.getRouter();

export default router;
