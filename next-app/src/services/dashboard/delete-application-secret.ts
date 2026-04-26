import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";
import { api } from "../base/gatekeeper-api";

type Request = {
  secretId: string;
  applicationId: string;
  tenantId?: string;
};

export async function deleteApplicationSecretApi(
  { secretId, applicationId, tenantId }: Request,
  { accessToken }: IServiceOptions
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete(
      `/v1/tenants/${tenantId}/applications/${applicationId}/secrets/${secretId}`,
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
