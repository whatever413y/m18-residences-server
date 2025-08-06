import { Router, Request, Response } from "express";
import BaseController from "../controllers/BaseController";

class BaseRoute<T> {
  private router: Router;

  constructor(private controller: BaseController<T>, private path: string) {
    this.router = Router();
    this.initializeRoutes();
  }

  private initializeRoutes() {
    this.router.get(`${this.path}`, this.controller.getAll);
    this.router.get(`${this.path}/tenant/:tenantId`, this.controller.getAllByTenantId);
    this.router.get(`${this.path}/tenant/name/:tenantName`, this.controller.getByTenantName);
    this.router.get(`${this.path}/:id`, this.controller.getById);
    this.router.post(`${this.path}`, this.controller.create);
    this.router.put(`${this.path}/:id`, this.controller.update);
    this.router.delete(`${this.path}/:id`, this.controller.delete);
  }

  public addCustomPost(
    endpoint: string,
    handler: (req: Request, res: Response) => void
  ) {
    this.router.post(`${this.path}${endpoint}`, handler);
  }

  public addCustomGet(endpoint: string, handler: (req: Request, res: Response) => void) {
    this.router.get(`${this.path}${endpoint}`, handler);
  }

  public getRouter() {
    return this.router;
  }
}

export default BaseRoute;
