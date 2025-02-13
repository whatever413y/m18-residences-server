import { Request, Response } from "express";
import TenantRepository from "../repository/TenantRepository";
import BaseController from "./BaseController";

class TenantController extends BaseController<typeof TenantRepository> {
   
    protected repository = new TenantRepository();

    
    protected getRepositoryName() {
        return "Tenant";
    }
}

export default TenantController;