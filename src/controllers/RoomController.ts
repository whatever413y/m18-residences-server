import RoomRepository from "../repository/RoomRepository";
import BaseController from "./BaseController";

class RoomController extends BaseController<typeof RoomRepository> {
   
    protected repository = new RoomRepository();
    
    protected getRepositoryName() {
        return "Room";
    }
}

export default RoomController;