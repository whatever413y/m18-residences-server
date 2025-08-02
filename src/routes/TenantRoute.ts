import TenantController from "../controllers/TenantController";
import BaseRoute from "./BaseRoute";

const TenantRoute = new BaseRoute(new TenantController(), "/api/tenants");
const router = TenantRoute.getRouter();

export default router;
