import BillController from "../controllers/BillController";
import BaseRoute from "./BaseRoute";

const billController = new BillController();
const BillRoutes = new BaseRoute(billController, "/bills");

BillRoutes.addCustomPost("/generate", billController.createBills);

export default BillRoutes.getRouter();
