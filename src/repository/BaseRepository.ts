import pool from "../config/Database";

interface BaseRepository {
  getAll<T>(): Promise<T[]>;
  getById<T>(id: number): Promise<T | null>;
  create<T>(fields: string[], values: T[]): Promise<T>;
  update<T>(id: number, fields: string[], values: T[]): Promise<T>;
  delete(id: number): Promise<void>;
}

// Base repository class
class BaseRepository implements BaseRepository {
  protected tableName: string;

  constructor() {
    this.tableName = "base_table";
  }

  // Get all records
  async getAll<T>(): Promise<T[]> {
    const result = await pool.query(`SELECT * FROM ${this.tableName}`);
    return result.rows;
  }

  // Get a record by ID
  async getById<T>(id: number): Promise<T | null> {
    const result = await pool.query(`SELECT * FROM ${this.tableName} WHERE id = $1`, [id]);
    return result.rows[0] || null;
  }

  // Create a new record
  async create<T>(fields: string[], values: T[]): Promise<T> {
    const fieldNames = fields.join(", ");
    const placeholders = fields.map((_, i) => `$${i + 1}`).join(", ");
    
    const query = `INSERT INTO ${this.tableName} (${fieldNames}) VALUES (${placeholders}) RETURNING *`;
    
    const result = await pool.query(query, values);
    return result.rows[0];
  }

  // Update a record
  async update<T>(id: number, fields: string[], values: T[]): Promise<T> {
    const updates = fields.map((field, i) => `${field} = $${i + 1}`).join(", ");
    const query = `UPDATE ${this.tableName} SET ${updates} WHERE id = $${fields.length + 1} RETURNING *`;

    const result = await pool.query(query, [...values, id]);
    return result.rows[0];
  }

  // Delete a record
  async delete(id: number): Promise<void> {
    const query = `DELETE FROM ${this.tableName} WHERE id = $1`;
    await pool.query(query, [id]);
  }
}

export default BaseRepository;
