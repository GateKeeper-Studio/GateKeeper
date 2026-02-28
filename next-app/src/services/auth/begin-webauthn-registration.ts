import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  userId: string;
  applicationId: string;
};

type Response = {
  sessionId: string;
  options: unknown;
};

export async function beginWebAuthnRegistrationApi({
  userId,
  applicationId,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(
      `/v1/auth/webauthn/begin-registration`,
      { userId, applicationId },
    );
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
