import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  newEmail: string;
  stepUpToken: string;
};

type Response = {
  message: string;
};

export async function accountRequestEmailChangeApi(
  { applicationId, newEmail, stepUpToken }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/account/email/change`,
      { applicationId, newEmail },
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
