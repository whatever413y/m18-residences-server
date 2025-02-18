import { Request, Response } from "express";
import BillRepository from "../repository/BillRepository";
import { Billing } from "../interfaces/Bill";
import BaseController from "./BaseController";

class BillController extends BaseController<typeof BillRepository> {
  protected repository = new BillRepository();

  protected getRepositoryName() {
    return "Bill";
  }

  createBills = async (req: Request, res: Response) => {
    const data: Billing[] = req.body as Billing[];
    try {
      const newItems = await this.repository.createBills(data);
      res.status(201).json(newItems);
      console.log(data)
    } catch (err) {
      res
        .status(500)
        .json({ error: `Failed to create ${this.getRepositoryName()}` });
      console.log("Error: ", err);
    }
  };
}

export default BillController;
