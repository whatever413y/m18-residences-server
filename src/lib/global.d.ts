import { JwtPayload } from "jsonwebtoken";
import { File as MulterFile } from "multer";

declare global {
  namespace Express {
    interface Request {
      user?: JwtPayload & {
        id: number;
        name: string;
      };
      file?: MulterFile;           
      files?: MulterFile[];       
    }
  }
}

export {};
