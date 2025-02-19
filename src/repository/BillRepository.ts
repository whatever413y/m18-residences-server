import BaseRepository from "./BaseRepository";
import pool from "../config/Database";
import { Billing } from "../interfaces/Bill";

class BillRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Bills";
  }

  async createBills<T>(data: Billing[]): Promise<T[]> {
    if (data.length === 0) return [];
  
    const fieldNames = [
      "tenant_id",
      "room_charges",
      "electric_charges",
      "additional_charges",
      "additional_description",
    ].join(", ");
    
    const placeholders: string[] = [];
    const values: any[] = [];
  
    data.forEach((bill, index) => {
      const start = index * 4; 
      
      placeholders.push(`(
        $${start + 1},  
        (SELECT COALESCE(r.rent, 0) FROM Rooms r WHERE r.id = (SELECT t.room_id FROM Tenants t WHERE t.id = $${start + 1})), 
        COALESCE((
          SELECT e.consumption * $${start + 4} FROM Electricity_Readings e
          WHERE e.tenant_id = $${start + 1}
          AND e.created_at = (SELECT MAX(created_at) FROM Electricity_Readings WHERE tenant_id = $${start + 1})
        ), 0),  -- electric_charges
        $${start + 2},  
        $${start + 3}  
      )`);
  
      values.push(
        bill.tenant_id,
        bill.additional_charges || 0,
        bill.additional_description || null,
        bill.rate
      );
    });
  
    const query = `
      INSERT INTO ${this.tableName} (${fieldNames})
      VALUES ${placeholders.join(", ")}
      RETURNING *;
    `;
  
    const result = await pool.query(query, values);
    return result.rows;
  }
  

}

export default BillRepository;
