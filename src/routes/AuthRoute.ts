import { Router } from "express";
import AuthController from "../controllers/AuthController";
import { authenticateToken } from "../middleware/authMiddleware";

const router = Router();
const controller = new AuthController();

router.post("/api/auth/admin-login", (req, res) => controller.adminLogin(req, res));
router.post("/api/auth/login", (req, res) => controller.login(req, res));
router.get("/api/auth/validate-token", authenticateToken, (req, res) => controller.validateToken(req, res));
router.get(
  "/api/auth/receipts/:tenantName/:filename",
  authenticateToken,
  (req, res) => controller.getReceiptSignedUrl(req, res)
);
router.get(
  "/api/auth/payments/:filename",
  authenticateToken,
  (req, res) => controller.getPaymentSignedUrl(req, res)
);

export default router;
