import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  tenantId: string;
};

export async function deleteTenantApi(
  { tenantId }: Request,
  { accessToken }: IServiceOptions
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete<Response>(`/v1/tenants/${tenantId}`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    return [null];
  } catch (error: unknown) {
    return [error as APIError];
  }
}
