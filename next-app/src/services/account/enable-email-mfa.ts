import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Response = {
  message: string;
};

export async function enableEmailMfaApi({
  accessToken,
}: IServiceOptions): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/account/mfa/enable-email`,
      {},
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
