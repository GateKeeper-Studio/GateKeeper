import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  password: string;
  totpCode?: string;
};

type Response = {
  stepUpToken: string;
  expiresIn: number;
};

export async function reauthenticateApi(
  { applicationId, password, totpCode }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/account/reauthenticate`,
      { applicationId, password, totpCode: totpCode || undefined },
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
