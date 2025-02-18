import { Request, Response } from "express";
import BaseRepository from "../repository/BaseRepository";

// Generic BaseController to handle CRUD operations
abstract class BaseController<T> {
  // Abstract method to get the model class that will handle DB operations
  protected abstract repository: BaseRepository;

  // Method to get all records
  getAll = async (req: Request, res: Response) => {
    try {
      const items = await this.repository.getAll();
      res.json(items);
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });      
      console.log("Error: ", err);
    }
  };

  // Method to get record by ID
  getById = async (req: Request, res: Response) => {
    const { id } = req.params;
    try {
      const item = await this.repository.getById(Number(id));
      if (item) {
        res.json(item);
      } else {
        res.status(404).json({ error: `${this.getRepositoryName()} not found` });
      }
    } catch (err) {
      res.status(500).json({ error: `Failed to fetch ${this.getRepositoryName()}` });
      console.log("Error: ", err);
    }
  };

  // Method to create a new record
  create = async (req: Request, res: Response) => {
    const data = req.body;
    try {
      const fields = Object.keys(data);
      const values = Object.values(data);
      const newItem = await this.repository.create(fields, values);
      res.status(201).json(newItem);
    } catch (err) {
      res.status(500).json({ error: `Failed to create ${this.getRepositoryName()}` });
      console.log("Error: ", err);
    }
  };

  // Method to update a record by ID
  update = async (req: Request, res: Response) => {
    const { id } = req.params;
    const data = req.body;
    try {
      const fields = Object.keys(data);
      const values = Object.values(data);
      const updatedItem = await this.repository.update(Number(id), fields, values);
      res.json(updatedItem);
    } catch (err) {
      res.status(500).json({ error: `Failed to update ${this.getRepositoryName()}` });
      console.log("Error: ", err);
    }
  };

  // Method to delete a record by ID
  delete = async (req: Request, res: Response) => {
    const { id } = req.params;
    try {
      await this.repository.delete(Number(id));
      res.status(204).send();
    } catch (err) {
      res.status(500).json({ error: `Failed to delete ${this.getRepositoryName()}` });
      console.log("Error: ", err);
    }
  };

  // Abstract method to return the name of the model for error messages
  protected abstract getRepositoryName(): string;
}

export default BaseController;
