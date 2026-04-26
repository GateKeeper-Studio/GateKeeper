import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

export interface IApplication {
  id: string;
  name: string;
  description: string;
  createdAt: Date;
  updatedAt: Date;
}

type Request = {
  tenantId: string;
};

type Response = IApplication;

export async function getTenantByIdService(
  { tenantId }: Request,
  { accessToken }: IServiceOptions
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(
      `/v1/tenants/${tenantId}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      }
    );

    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
