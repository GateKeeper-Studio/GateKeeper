import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

type Request = {
  firstName: string;
  lastName: string;
  displayName: string;
  phoneNumber: string | null;
  address: string | null;
};

type Response = {
  firstName: string;
  lastName: string;
  displayName: string;
  phoneNumber: string | null;
  address: string | null;
};

export async function updateProfileApi(
  request: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.put<Response>(`/v1/account/profile`, request, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
