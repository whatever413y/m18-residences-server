import { Request, Response } from "express";
import jwt from "jsonwebtoken";
import TenantRepository from "../repository/TenantRepository";

const JWT_SECRET = process.env.JWT_SECRET || "supersecret";

class AuthController {
  private tenantRepo = new TenantRepository();

  async login(req: Request, res: Response) {
    const { name } = req.body;

    if (!name) {
      return res.status(400).json({ error: "Tenant name is required" });
    }

    try {
      const tenant = await this.tenantRepo.getByTenantName(name);

      if (!tenant) {
        return res.status(404).json({ error: "Tenant not found" });
      }

      const token = jwt.sign({ id: tenant.id, name: tenant.name }, JWT_SECRET, {
        expiresIn: "10m",
      });

      return res.json({ token, tenant });
    } catch (error) {
      console.error("Login error:", error);
      return res.status(500).json({ error: "Internal server error" });
    }
  }
  async validateToken(req: Request, res: Response) {
    const user = (req as any).user;
    return res.status(200).json({ valid: true, user });
  }
}

export default AuthController;
