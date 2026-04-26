import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  id: string;
  name: string;
  description?: string;
};

type Response = {
  id: string;
  name: string;
  description?: string;
};

export async function editTenantApi(
  { id, name, description }: Request,
  { accessToken }: IServiceOptions
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.put<Response>(
      `/v1/tenants/${id}`,
      {
        name,
        description,
      },
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
