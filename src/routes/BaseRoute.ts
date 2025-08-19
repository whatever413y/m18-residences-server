import { Router, Request, Response } from "express";
import BaseController from "../controllers/BaseController";
import { authenticate, requireAdmin } from "../middleware/authMiddleware";
import multer from 'multer';

const upload = multer();

class BaseRoute<T> {
  private router: Router;

  constructor(private controller: BaseController<T>, private path: string) {
    this.router = Router();
    this.initializeRoutes();
  }

  private initializeRoutes() {
    this.router.get(`${this.path}`, authenticate, this.controller.getAll);
    this.router.get(`${this.path}/:id`, authenticate, this.controller.getById);

    // Admin routes
    this.router.post(
      `${this.path}`,
      authenticate,
      requireAdmin,
      this.controller.create
    );
    this.router.put(`${this.path}/:id`, authenticate, requireAdmin, this.controller.update);
    this.router.delete(
      `${this.path}/:id`, authenticate,
      requireAdmin,
      this.controller.delete
    );
  }

  public addCustomPost(
    endpoint: string,
    handler: (req: Request, res: Response) => void,
    protectedRoute = true
  ) {
    if (protectedRoute) {
      this.router.post(`${this.path}${endpoint}`, authenticate, handler);
    } else {
      this.router.post(`${this.path}${endpoint}`, handler);
    }
  }

  public addCustomGet(
    endpoint: string,
    handler: (req: Request, res: Response) => void,
    protectedRoute = true
  ) {
    if (protectedRoute) {
      this.router.get(`${this.path}${endpoint}`, authenticate, handler);
    } else {
      this.router.get(`${this.path}${endpoint}`, handler);
    }
  }

  public getRouter() {
    return this.router;
  }
}

export default BaseRoute;
