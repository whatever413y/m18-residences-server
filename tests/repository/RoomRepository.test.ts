import RoomRepository from "../../src/repository/RoomRepository"
import pool from "../../src/config/Database"

jest.mock('../../src/config/Database', () => ({
  query: jest.fn() as jest.Mock, // Cast query to jest.Mock
}));

describe('RoomRepository', () => {
  const roomRepository = new RoomRepository();
  const tableName = 'Rooms';

  afterEach(() => {
    jest.clearAllMocks();
  });

  describe('getAll', () => {
    it('should return all records', async () => {
      const mockRows = [{ id: 1, name: 'Test' }];
      (pool.query as jest.Mock).mockResolvedValue({ rows: mockRows });

      const result = await roomRepository.getAll();

      expect(pool.query).toHaveBeenCalledWith(`SELECT * FROM ${tableName}`);
      expect(result).toEqual(mockRows);
    });
  });

  describe('getById', () => {
    it('should return a record by ID', async () => {
      const mockRow = { id: 1, name: 'Test' };
      (pool.query as jest.Mock).mockResolvedValue({ rows: [mockRow] });

      const result = await roomRepository.getById(1);

      expect(pool.query).toHaveBeenCalledWith(`SELECT * FROM ${tableName} WHERE id = $1`, [1]);
      expect(result).toEqual(mockRow);
    });

    it('should return null if no record is found', async () => {
      (pool.query as jest.Mock).mockResolvedValue({ rows: [] });

      const result = await roomRepository.getById(1);

      expect(pool.query).toHaveBeenCalledWith(`SELECT * FROM ${tableName} WHERE id = $1`, [1]);
      expect(result).toBeNull();
    });
  });

  describe('create', () => {
    it('should create a new record', async () => {
      const fields = ['name'];
      const values = ['Test'];
      const mockRow = { id: 1, name: 'Test' };
      (pool.query as jest.Mock).mockResolvedValue({ rows: [mockRow] });

      const result = await roomRepository.create(fields, values);

      expect(pool.query).toHaveBeenCalledWith(
        `INSERT INTO ${tableName} (name) VALUES ($1) RETURNING *`,
        values
      );
      expect(result).toEqual(mockRow);
    });
  });

  describe('update', () => {
    it('should update a record', async () => {
      const fields = ['name'];
      const values = ['Updated Test'];
      const mockRow = { id: 1, name: 'Updated Test' };
      (pool.query as jest.Mock).mockResolvedValue({ rows: [mockRow] });

      const result = await roomRepository.update(1, fields, values);

      expect(pool.query).toHaveBeenCalledWith(
        `UPDATE ${tableName} SET name = $1 WHERE id = $2 RETURNING *`,
        [...values, 1]
      );
      expect(result).toEqual(mockRow);
    });
  });

  describe('delete', () => {
    it('should delete a record', async () => {
      (pool.query as jest.Mock).mockResolvedValue({});

      await roomRepository.delete(1);

      expect(pool.query).toHaveBeenCalledWith(`DELETE FROM ${tableName} WHERE id = $1`, [1]);
    });
  });
});
