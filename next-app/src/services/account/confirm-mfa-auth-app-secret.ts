import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  userId: string;
  mfaAuthAppCode: string;
};

export async function confirmMfaAuthAppSecretApi(
  { userId, mfaAuthAppCode }: Request,
  { accessToken }: IServiceOptions,
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.post(
      `/v1/auth/confirm-mfa-auth-app-secret`,
      { userId, mfaAuthAppCode },
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      },
    );
    return [null];
  } catch (error: unknown) {
    return [error as APIError];
  }
}
