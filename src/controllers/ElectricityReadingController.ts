import { Request, Response } from "express";
import ElectricityReadingRepository from "../repository/ElectricityReadingRepository";
import BaseController from "./BaseController";
import ElectricityReading from "../interfaces/ElectricityReading";

class ElectricityReadingController extends BaseController<typeof ElectricityReadingRepository> {
   
    protected repository = new ElectricityReadingRepository();
    
    protected getRepositoryName() {
        return "Electricity_Reading";
    }
}

export default ElectricityReadingController;