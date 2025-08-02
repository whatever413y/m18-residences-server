import BillController from "../controllers/BillController";
import BaseRoute from "./BaseRoute";

const billController = new BillController();
const BillRoutes = new BaseRoute(billController, "/bills");

export default BillRoutes.getRouter();
