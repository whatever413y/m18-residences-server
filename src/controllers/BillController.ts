import { Request, Response } from "express";
import { Bill } from "@prisma/client";
import BillRepository from "../repository/BillRepository";
import BaseController from "./BaseController";
import { uploadFile } from "../middleware/r2";
import TenantRepository from "../repository/TenantRepository";

class BillController extends BaseController<Bill> {
  protected repository = new BillRepository();
  protected tenantRepo = new TenantRepository();

  update = async (req: Request, res: Response) => {
    try {
      let receiptUrl: string | undefined;

      const tenant = await this.tenantRepo.getById(Number(req.body.tenantId));

      if (req.file) {
        const fullUrl = await uploadFile(
          `receipts/${tenant?.name}/${Date.now()}-r${req.body.readingId}`,
          req.file.buffer,
          req.file.mimetype
        );
        const urlParts = fullUrl.split("/");
        const filename = urlParts[urlParts.length - 1];
        receiptUrl = filename;
      }

      let additionalChargesParsed;
      if (req.body.additionalCharges) {
        if (typeof req.body.additionalCharges === "string") {
          try {
            additionalChargesParsed = JSON.parse(req.body.additionalCharges);
          } catch (e) {
            console.error("Failed to parse additionalCharges:", e);
            return;
          }
        } else {
          additionalChargesParsed = req.body.additionalCharges;
        }
      }

      const updateData = {
        ...req.body,
        ...(additionalChargesParsed && {
          additionalCharges: additionalChargesParsed,
        }),
        ...(receiptUrl && { receiptUrl }),
      };

      updateData.tenantId = Number(updateData.tenantId);
      updateData.readingId = Number(updateData.readingId);
      updateData.roomCharges = Number(updateData.roomCharges);
      updateData.electricCharges = Number(updateData.electricCharges);

      const updatedItem = await this.repository.update(
        Number(req.params.id),
        updateData
      );
      res.json(updatedItem);
    } catch (err) {
      console.error(err);
      res.status(500).json({ error: "Failed to update bill" });
    }
  };

  protected getRepositoryName() {
    return "Bill";
  }
}

export default BillController;
