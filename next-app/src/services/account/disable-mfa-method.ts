import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  method: string;
  stepUpToken: string;
};

type Response = {
  message: string;
};

export async function disableMfaMethodApi(
  { method, stepUpToken }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.delete<Response>(
      `/v1/account/mfa/methods/${method}`,
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
