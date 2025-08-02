import TenantRepository from "../repository/TenantRepository";
import BaseController from "./BaseController";
import { Tenant } from "@prisma/client";

class TenantController extends BaseController<Tenant> {
  protected repository = new TenantRepository();

  protected getRepositoryName() {
    return "Tenant";
  }
}

export default TenantController;
