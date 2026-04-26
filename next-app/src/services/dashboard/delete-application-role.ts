import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  applicationId: string;
  tenantId: string;
  roleId: string;
};

export async function deleteApplicationRoleApi(
  { applicationId, tenantId, roleId }: Request,
  { accessToken }: IServiceOptions
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete<Response>(
      `/v1/tenants/${tenantId}/applications/${applicationId}/roles/${roleId}`,
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
