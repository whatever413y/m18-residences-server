abstract class BaseRepository<T> {
  abstract getAll(): Promise<T[]>;
  abstract getById(id: number): Promise<T | null>;
  abstract create(data: any): Promise<T>;
  abstract update(id: number, data: any): Promise<T>;
  abstract delete(id: number): Promise<T>;
}

export default BaseRepository;
