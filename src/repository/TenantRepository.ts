import pool from "../config/Database";
import BaseRepository from "./BaseRepository";

class TenantRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Tenants";
  }

  async getAll<T>(): Promise<T[]> {
    const result = await pool.query(`
        SELECT 
            ${this.tableName}.id, 
            Rooms.name AS room_name, 
            ${this.tableName}.name AS tenant_name, 
            ${this.tableName}.created_at, 
            ${this.tableName}.updated_at
        FROM ${this.tableName}
        JOIN Rooms ON ${this.tableName}.room_id = Rooms.id
    `);
    return result.rows;
}



}

export default TenantRepository;
