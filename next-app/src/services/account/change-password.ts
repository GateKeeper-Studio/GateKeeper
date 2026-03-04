import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  currentPassword: string;
  newPassword: string;
  stepUpToken: string;
};

type Response = {
  message: string;
};

export async function accountChangePasswordApi(
  { applicationId, currentPassword, newPassword, stepUpToken }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/account/change-password`,
      { applicationId, currentPassword, newPassword },
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
          "X-Step-Up-Token": stepUpToken,
        },
      },
    );
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
