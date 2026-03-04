import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  stepUpToken: string;
};

type Response = {
  message: string;
};

export async function accountRevokeAllSessionsApi(
  { applicationId, stepUpToken }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.delete<Response>(`/v1/account/sessions`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "X-Step-Up-Token": stepUpToken,
      },
      data: { applicationId },
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
