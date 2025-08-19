import { Request, Response } from "express";
import { Bill } from "@prisma/client";
import BillRepository from "../repository/BillRepository";
import BaseController from "./BaseController";
import { uploadFile } from "../middleware/r2";
import TenantRepository from "../repository/TenantRepository";

class BillController {
  protected repository = new BillRepository();
  protected tenantRepo = new TenantRepository();

  getAll = async (req: Request, res: Response) => {
    try {
      const items = await this.repository.getAll();
      res.json(items);
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  getAllById = async (req: Request, res: Response) => {
    const id = Number(req.params.id);
    try {
      const items = await this.repository.getAllById!(id);
      res.json(items);
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  getById = async (req: Request, res: Response) => {
    const id = Number(req.params.id);
    try {
      const item = await this.repository.getById(id);
      if (!item) return res.status(404).json({ error: `${this.getRepositoryName()} not found` });
      res.json(item);
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  getByTenantId = async (req: Request, res: Response) => {
    const id = Number(req.params.id);
    try {
      const item = await this.repository.getByTenantId!(id);
      if (!item) return res.status(404).json({ error: `${this.getRepositoryName()} not found` });
      res.json(item);
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  create = async (req: Request, res: Response) => {
    try {
      const newItem = await this.repository.create(req.body);
      res.status(201).json(newItem);
    } catch (err) {
      res.status(500).json({ error: `Failed to create ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  update = async (req: Request, res: Response) => {
    try {
      let receipt_url: string | undefined;

      const tenant = await this.tenantRepo.getById(Number(req.body.tenant_id));

      if (req.file) {
        const fullUrl = await uploadFile(
          `receipts/${tenant?.name}/${Date.now()}-r${req.body.reading_id}`,
          req.file.buffer,
          req.file.mimetype
        );
        const urlParts = fullUrl.split("/");
        const filename = urlParts[urlParts.length - 1];
        receipt_url = filename;
      }

      let additionalChargesParsed;
      if (req.body.additional_charges) {
        if (typeof req.body.additional_charges === "string") {
          try {
            additionalChargesParsed = JSON.parse(req.body.additional_charges);
          } catch (e) {
            console.error("Failed to parse additionalCharges:", e);
            return;
          }
        } else {
          additionalChargesParsed = req.body.additional_charges;
        }
      }

      const updateData = {
        ...req.body,
        ...(additionalChargesParsed && {
          additional_charges: additionalChargesParsed,
        }),
        ...(receipt_url && { receipt_url }),
      };

      updateData.tenant_id = Number(updateData.tenant_id);
      updateData.reading_id = Number(updateData.reading_id);
      updateData.room_charges = Number(updateData.room_charges);
      updateData.electric_charges = Number(updateData.electric_charges);

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

  delete = async (req: Request, res: Response) => {
    const id = Number(req.params.id);
    try {
      await this.repository.delete(id);
      res.status(204).send();
    } catch (err) {
      res.status(500).json({ error: `Failed to delete ${this.getRepositoryName()}` });
      console.error(err);
    }
  };

  protected getRepositoryName() {
    return "Bill";
  }
}

export default BillController;
