import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  tenantId: string;
  userId: string;
};

type Response = UserByIdResponse;

export type UserByIdResponse = {
  id: string;
  displayName: string;
  firstName: string;
  lastName: string;
  email: string;
  isActive: boolean;
  tenantId: string;
  tenantName: string;
  address: string | null;
  photoUrl: string | null;
  isMfaEmailConfigured: boolean;
  isMfaAuthAppConfigured: boolean;
  isMfaWebauthnConfigured: boolean;
  preferred2FAMethod: "email" | "sms" | "totp" | "webauthn" | null;
  isEmailVerified: boolean;
  badges: {
    id: string;
    name: string;
  }[];
};

export async function getTenantUserByIdService(
  { userId, tenantId }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(
      `/v1/tenants/${tenantId}/users/${userId}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      },
    );

    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
