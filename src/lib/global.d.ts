import { JwtPayload } from "jsonwebtoken";

declare global {
  namespace Express {
    interface Request {
      user?: JwtPayload & {
        id: number;
        name: string;
      };
    }
  }
}

export {};
