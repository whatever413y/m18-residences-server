import { Bill } from "@prisma/client";
import BillRepository from "../repository/BillRepository";
import BaseController from "./BaseController";

class BillController extends BaseController<Bill> {
  protected repository = new BillRepository();

  protected getRepositoryName() {
    return "Bill";
  }
}

export default BillController;
