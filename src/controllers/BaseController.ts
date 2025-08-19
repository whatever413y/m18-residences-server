import { Request, Response } from "express";
import BaseRepository from "../repository/BaseRepository";

abstract class BaseController<T> {
  protected abstract repository: BaseRepository<T>;

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

  getByTenantName = async (req: Request, res: Response) => {
    const tenantName = req.params.tenantName;
    try {
      const item = await this.repository.getByTenantName!(tenantName);
      res.json(item);
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
    const id = Number(req.params.id);
    try {
      const updatedItem = await this.repository.update(id, req.body);
      res.json(updatedItem);
    } catch (err) {
      res.status(500).json({ error: `Failed to update ${this.getRepositoryName()}` });
      console.error(err);
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

  protected abstract getRepositoryName(): string;
}

export default BaseController;
