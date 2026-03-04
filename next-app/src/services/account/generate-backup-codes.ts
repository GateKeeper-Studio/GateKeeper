import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  stepUpToken: string;
};

type Response = {
  codes: string[];
  message: string;
};

export async function accountGenerateBackupCodesApi(
  { applicationId, stepUpToken }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/account/backup-codes`,
      { applicationId },
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
