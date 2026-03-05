import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  organizationId: string;
  userId: string;
};

export async function deleteTenantUserApi(
  { organizationId, userId }: Request,
  { accessToken }: IServiceOptions,
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete<Response>(
      `/v1/organizations/${organizationId}/users/${userId}`,
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
