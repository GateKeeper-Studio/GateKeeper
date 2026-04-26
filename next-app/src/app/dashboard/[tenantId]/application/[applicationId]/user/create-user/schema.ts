import { z } from "zod";

export const formSchema = z.object({
  displayName: z.string().min(2).max(50),
  firstName: z.string().min(2).max(50),
  lastName: z.string().min(2).max(50),
  email: z.string().email(),
  isEmailConfirmed: z.boolean(),
  hasMfaEmailEnabled: z.boolean(),
  hasMfaAuthAppEnabled: z.boolean(),
  roles: z.array(z.string()),
  temporaryPassword: z.string().min(8).max(50),
});
