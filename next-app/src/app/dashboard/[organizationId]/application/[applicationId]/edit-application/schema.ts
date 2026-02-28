import { z } from "zod";

export const formSchema = z.object({
  name: z
    .string()
    .min(2, "Name must hast at least 2 characters.")
    .max(75, "Name must have at most 75 characters."),

  description: z.string().optional(),

  badges: z.array(z.string()),
  hasMfaAuthApp: z.boolean(),
  hasMfaEmail: z.boolean(),
  hasMfaWebauthn: z.boolean(),
  canSelfSignUp: z.boolean(),
  canSelfForgotPass: z.boolean(),
});
