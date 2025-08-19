import { Router } from "express";
import BillController from "../controllers/BillController";
import { authenticate, requireAdmin } from "../middleware/authMiddleware";
import multer from 'multer';

const upload = multer();
const billController = new BillController();

class BillRoute {
  private router: Router;

  constructor(private controller: BillController, private path: string) {
    this.router = Router();
    this.initializeRoutes();
  }

  private initializeRoutes() {
    this.router.get(`${this.path}`, authenticate, this.controller.getAll);
    this.router.get(`${this.path}/:id`, authenticate, this.controller.getById);

    //Bill routes
    this.router.get(
      `${this.path}/:id/bills`,
      authenticate,
      this.controller.getAllById
    );
    this.router.get(`${this.path}/:id/bill`, authenticate, this.controller.getByTenantId);

    // Admin routes
    this.router.post(
      `${this.path}`,
      authenticate,
      requireAdmin,
      this.controller.create
    );
    this.router.put(`${this.path}/:id`, authenticate, requireAdmin, this.controller.update);
    this.router.put(`${this.path}/:id/upload`, authenticate, requireAdmin, upload.single("receipt_file"), this.controller.update);

    this.router.delete(
      `${this.path}/:id`, authenticate,
      requireAdmin,
      this.controller.delete
    );
  }

  public getRouter() {
    return this.router;
  }
}

const BillRoutes = new BillRoute(billController, "/api/bills");

export default BillRoutes.getRouter();
