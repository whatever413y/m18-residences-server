import RoomController from "../controllers/RoomController";
import BaseRoute from "./BaseRoute";

const RoomRoutes = new BaseRoute(new RoomController(), "/api/rooms");
const router = RoomRoutes.getRouter();

export default router;
