import BaseRepository from "./BaseRepository";

class BillRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Bills";
  }

}

export default BillRepository;
