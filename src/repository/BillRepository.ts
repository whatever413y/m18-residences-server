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
      const start = index * 4;  // Adjusted to accommodate all 5 fields
      
      placeholders.push(`(
        $${start + 1},  -- tenant_id
        (SELECT COALESCE(r.rent, 0) FROM Rooms r WHERE r.id = (SELECT t.room_id FROM Tenants t WHERE t.id = $${start + 1})),  -- room_charges
        COALESCE((
          SELECT e.consumption * $${start + 4} FROM Electricity_Readings e
          WHERE e.tenant_id = $${start + 1}
          AND e.created_at = (SELECT MAX(created_at) FROM Electricity_Readings WHERE tenant_id = $${start + 1})
        ), 0),  -- electric_charges
        $${start + 2},  -- additional_charges
        $${start + 3}   -- additional_description
      )`);
  
      // Push values for each entry
      values.push(
        bill.tenant_id,
        bill.additional_charges || 0,
        bill.additional_description || null,
        bill.rate
      );
    });
  
    // Now construct the full insert query
    const query = `
      INSERT INTO ${this.tableName} (${fieldNames})
      VALUES ${placeholders.join(", ")}
      RETURNING *;
    `;
  
    // Execute the query
    const result = await pool.query(query, values);
    return result.rows;
  }
  

}

export default BillRepository;
