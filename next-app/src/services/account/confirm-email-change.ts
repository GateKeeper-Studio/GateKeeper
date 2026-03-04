import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  token: string;
};

type Response = {
  message: string;
};

export async function accountConfirmEmailChangeApi({
  token,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(`/v1/account/email/confirm`, {
      token,
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
