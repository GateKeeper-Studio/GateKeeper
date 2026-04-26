import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  applicationId: string;
  tenantId?: string;
};

export async function deleteApplicationApi(
  { applicationId, tenantId }: Request,
  { accessToken }: IServiceOptions
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete<Response>(
      `/v1/tenants/${tenantId}/applications/${applicationId}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      }
    );
    return [null];
  } catch (error: unknown) {
    return [error as APIError];
  }
}
