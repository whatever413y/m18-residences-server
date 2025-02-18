import BaseRepository from "./BaseRepository";

class ElectricityReadingRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Electricity_Readings";
  }

}

export default ElectricityReadingRepository;
