import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
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

type Response = {
  id: string;
  displayName: string;
  email: string;
  roles: { id: string; name: string; description: string }[];
};

export async function createTenantUserApi(
  {
    displayName,
    email,
    firstName,
    isEmailConfirmed,
    isMfaAuthAppEnabled,
    isMfaEmailEnabled,
    lastName,
    roles,
    temporaryPasswordHash,
    applicationId,
    tenantId,
  }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/tenants/${tenantId}/users`,
      {
        displayName,
        email,
        firstName,
        isEmailConfirmed,
        isMfaAuthAppEnabled,
        isMfaEmailEnabled,
        lastName,
        roles,
        temporaryPasswordHash,
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
