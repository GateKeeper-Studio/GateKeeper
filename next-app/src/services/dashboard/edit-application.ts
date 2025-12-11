import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  id: string;
  name: string;
  description?: string;
  badges: string[];
  hasMfaEmail: boolean;
  hasMfaAuthApp: boolean;
  organizationId: string;
  isActive: boolean;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
};

type Response = {
  id: string;
  name: string;
  description?: string;
  badges: string[];
  hasMfaEmail: boolean;
  hasMfaAuthApp: boolean;
  isActive: boolean;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
};

export async function editApplicationApi(
  {
    id,
    name,
    description,
    badges,
    hasMfaAuthApp,
    hasMfaEmail,
    organizationId,
    isActive,
    canSelfForgotPass,
    canSelfSignUp,
  }: Request,
  { accessToken }: IServiceOptions
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.put<Response>(
      `/v1/organizations/${organizationId}/applications/${id}`,
      {
        id,
        name,
        description: description || null,
        badges,
        hasMfaAuthApp,
        hasMfaEmail,
        organizationId,
        isActive,
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
