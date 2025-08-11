import { Request, Response } from "express";
import jwt from "jsonwebtoken";
import TenantRepository from "../repository/TenantRepository";

const JWT_SECRET = process.env.JWT_SECRET || "supersecret";

class AuthController {
  private tenantRepo = new TenantRepository();

  async adminLogin(req: Request, res: Response) {
    const { username, password } = req.body;

    const adminUsername = process.env.ADMIN_USERNAME;
    const adminPassword = process.env.ADMIN_PASSWORD;

    if (!username || !password) {
      return res
        .status(400)
        .json({ error: "Username and password are required" });
    }

    if (username !== adminUsername || password !== adminPassword) {
      return res.status(401).json({ error: "Invalid admin credentials" });
    }

    const token = jwt.sign({ role: "admin", username }, JWT_SECRET, {
      expiresIn: "1h",
    });

    return res.status(200).json({ token, user: { role: "admin", username } });
  }

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
      return res.status(500).json({ error: "Internal server error" });
    }
  }
  async validateToken(req: Request, res: Response) {
    const user = (req as any).user;
    return res.status(200).json({ valid: true, user });
  }
}

export default AuthController;
