import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  tenantId: string;
  userId: string;
};

export async function deleteTenantUserApi(
  { tenantId, userId }: Request,
  { accessToken }: IServiceOptions,
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete<Response>(
      `/v1/tenants/${tenantId}/users/${userId}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      },
    );
    return [null];
  } catch (error: unknown) {
    return [error as APIError];
  }
}
