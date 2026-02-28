import { z } from "zod";

export const formSchema = z.object({
  displayName: z.string().min(2).max(50),
  firstName: z.string().min(2).max(50),
  lastName: z.string().min(2).max(50),
  email: z.string().email(),
  isEmailConfirmed: z.boolean(),
  preferred2FAMethod: z.enum(["totp", "email", "sms", "webauthn"]).nullable(),
  roles: z.array(z.string()),
  temporaryPassword: z.string(),
  isActive: z.boolean(),

  isMfaEmailConfigured: z.boolean(),
  IsMfaAuthAppConfigured: z.boolean(),
  isMfaWebauthnConfigured: z.boolean(),
});
