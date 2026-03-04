import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

export type MfaMethodItem = {
  type: string;
  enabled: boolean;
};

type Response = {
  preferredMethod: string | null;
  methods: MfaMethodItem[];
};

export async function listMfaMethodsApi({
  accessToken,
}: IServiceOptions): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(`/v1/account/mfa/methods`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
