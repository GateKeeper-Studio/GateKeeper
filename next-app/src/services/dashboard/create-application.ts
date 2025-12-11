import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  name: string;
  description?: string;
  passwordHashSecret: string;
  badges: string[];
  hasMfaEmail: boolean;
  hasMfaAuthApp: boolean;
  organizationId: string;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
};

type Response = {
  id: string;
  name: string;
  description?: string;
  passwordHashSecret: string;
  badges: string[];
  hasMfaEmail: boolean;
  hasMfaAuthApp: boolean;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
};

export async function createApplicationApi(
  {
    name,
    description,
    badges,
    hasMfaAuthApp,
    hasMfaEmail,
    passwordHashSecret,
    organizationId,
    canSelfForgotPass,
    canSelfSignUp,
  }: Request,
  { accessToken }: IServiceOptions
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/organizations/${organizationId}/applications`,
      {
        name,
        description: description || null,
        badges,
        hasMfaAuthApp,
        hasMfaEmail,
        passwordHashSecret,
        organizationId,
        canSelfForgotPass,
        canSelfSignUp,
      },
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      }
    );
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
