import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  organizationId?: string;
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
  applicationId: string;
  applicationName: string;
  address: string | null;
  photoUrl: string | null;
  isMfaEmailConfigured: boolean;
  isMfaAuthAppConfigured: boolean;
  preferred2FAMethod: "email" | "sms" | "totp" | null;
  isEmailVerified: boolean;
  badges: {
    id: string;
    name: string;
  }[];
};

export async function getApplicationUserByIdService(
  { applicationId, userId, organizationId }: Request,
  { accessToken }: IServiceOptions
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(
      `/v1/organizations/${organizationId}/applications/${applicationId}/users/${userId}`,
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
