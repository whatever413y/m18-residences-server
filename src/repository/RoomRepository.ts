import BaseRepository from "./BaseRepository";

class RoomRepository extends BaseRepository {
  constructor() {
    super();
    this.tableName = "Rooms";
  }

}

export default RoomRepository;
