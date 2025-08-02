import { ElectricityReading } from "@prisma/client";
import ElectricityReadingRepository from "../repository/ElectricityReadingRepository";
import BaseController from "./BaseController";

class ElectricityReadingController extends BaseController<ElectricityReading> {
    protected repository = new ElectricityReadingRepository();
    
    protected getRepositoryName() {
        return "Electricity_Reading";
    }
}

export default ElectricityReadingController;