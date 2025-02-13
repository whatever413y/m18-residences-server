import { Router } from "express";
import BaseController from "../controllers/BaseController";

class BaseRoute<T> {
  constructor(private controller: BaseController<T>, private path: string) {}

  getRouter() {
    const router = Router();

    router.get(`${this.path}`, this.controller.getAll);
    router.get(`${this.path}/:id`, this.controller.getById);
    router.post(`${this.path}`, this.controller.create);
    router.put(`${this.path}/:id`, this.controller.update);
    router.delete(`${this.path}/:id`, this.controller.delete);

    return router;
  }
}

export default BaseRoute;
