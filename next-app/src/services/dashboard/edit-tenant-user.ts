import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  userId: string;
  applicationId: string;
  tenantId: string;
  displayName: string;
  firstName: string;
  lastName: string;
  email: string;
  isEmailConfirmed: boolean;
  temporaryPasswordHash: string | null;
  roles: string[];
  isActive: boolean;
  preferred2FAMethod: string | null;
  // isMfaAuthAppEnabled: boolean;
  // isMfaEmailEnabled: boolean;
};

type Response = {
  id: string;
  displayName: string;
  firstName: string;
  lastName: string;
  email: string;
  isEmailConfirmed: boolean;
  temporaryPasswordHash: string;
  isMfaAuthAppEnabled: boolean;
  isMfaEmailEnabled: boolean;
  roles: string[];
  applicationId: string;
  tenantId: string;
};

export async function editTenantUserApi(
  {
    userId,
    applicationId,
    displayName,
    firstName,
    lastName,
    email,
    isEmailConfirmed,
    preferred2FAMethod,
    // isMfaAuthAppEnabled,
    // isMfaEmailEnabled,
    temporaryPasswordHash,
    roles,
    tenantId,
    isActive,
  }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.put<Response>(
      `/v1/tenants/${tenantId}/users/${userId}`,
      {
        displayName,
        firstName,
        lastName,
        email,
        isEmailConfirmed,
        preferred2FAMethod,
        // isMfaAuthAppEnabled,
        // isMfaEmailEnabled,
        temporaryPasswordHash,
        roles,
        isActive,
      },
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
