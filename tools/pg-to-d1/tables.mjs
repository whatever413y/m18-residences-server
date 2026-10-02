// The tables in foreign-key order (parents first), with their columns and how to convert each value.
// Must match the D1 schema (crates/db/migrations); verify.mjs compares counts, max ids and money sums.
export const TABLES = [
  { name: 'room', columns: { id: 'int', name: 'text', rent: 'int', created_at: 'ts', updated_at: 'ts' }, sums: ['rent'] },
  {
    name: 'tenant',
    columns: { id: 'int', room_id: 'int', name: 'text', is_active: 'bool', join_date: 'ts', created_at: 'ts', updated_at: 'ts' },
    sums: [],
  },
  {
    name: 'electricity_reading',
    columns: {
      id: 'int',
      tenant_id: 'int',
      room_id: 'int',
      prev_reading: 'int',
      curr_reading: 'int',
      consumption: 'int',
      created_at: 'ts',
      updated_at: 'ts',
    },
    sums: ['consumption'],
  },
  {
    name: 'bill',
    columns: {
      id: 'int',
      reading_id: 'int',
      tenant_id: 'int',
      room_charges: 'int',
      electric_charges: 'int',
      total_amount: 'int',
      receipt_url: 'text',
      paid: 'bool',
      created_at: 'ts',
      updated_at: 'ts',
    },
    sums: ['room_charges', 'electric_charges', 'total_amount'],
  },
  {
    name: 'additional_charge',
    columns: { id: 'int', bill_id: 'int', amount: 'int', description: 'text', created_at: 'ts', updated_at: 'ts' },
    sums: ['amount'],
  },
];

export const psql = process.env.PSQL ?? 'C:\\Program Files\\PostgreSQL\\18\\bin\\psql.exe';
