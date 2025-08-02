import RoomRepository from "../repository/RoomRepository";
import BaseController from "./BaseController";
import { Room } from "@prisma/client";

class RoomController extends BaseController<Room> {
  protected repository = new RoomRepository();

  protected getRepositoryName() {
    return "Room";
  }
}

export default RoomController;
