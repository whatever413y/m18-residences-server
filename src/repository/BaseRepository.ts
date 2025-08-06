abstract class BaseRepository<T> {
  abstract getAll(): Promise<T[]>;
  abstract getById(id: number): Promise<T | null>;
  abstract create(data: T): Promise<T>;
  abstract update(id: number, data: T): Promise<T>;
  abstract delete(id: number): Promise<T>;

  getAllByTenantId?(tenantId: number): Promise<T[]>;
  getByTenantName?(tenantName: string): Promise<T | null>;
}

export default BaseRepository;
