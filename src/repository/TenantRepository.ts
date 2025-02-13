import BaseRepository from "./BaseRepository";

class TenantRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Tenants";
  }

}

export default TenantRepository;
