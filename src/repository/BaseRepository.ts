abstract class BaseRepository<TModel, TCreate = Partial<TModel>, TUpdate = Partial<TModel>> {
  abstract getAll(): Promise<TModel[]>;
  abstract getById(id: number): Promise<TModel | null>;
  abstract create(data: TCreate): Promise<TModel>;
  abstract update(id: number, data: TUpdate): Promise<TModel>;
  abstract delete(id: number): Promise<TModel>;

  getAllById?(id: number): Promise<TModel[]>;
  getByTenantId?(id: number): Promise<TModel | null>;
  getByTenantName?(tenantName: string): Promise<TModel | null>;
}

export default BaseRepository;
