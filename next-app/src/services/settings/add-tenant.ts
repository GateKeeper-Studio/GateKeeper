import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  name: string;
  description?: string;
  passwordHashSecret: string;
};

type Response = {
  id: string;
  name: string;
  description?: string;
  passwordHashSecret: string;
};

export async function addTenantApi(
  { name, description, passwordHashSecret }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/tenants`,
      {
        name,
        description,
        passwordHashSecret,
      },
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
