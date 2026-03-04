import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Response = {
  otpUrl: string;
  expiresAt: string;
};

export async function getLastMfaTotpSecretApi({
  accessToken,
}: IServiceOptions): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(`/v1/account/mfa/totp-secret`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
