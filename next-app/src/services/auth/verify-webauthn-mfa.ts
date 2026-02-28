import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  email: string;
  sessionId: string;
  assertionData: unknown;
  applicationId: string;
};

type Response = {
  sessionCode: string;
};

export async function verifyWebAuthnMfaApi({
  email,
  sessionId,
  assertionData,
  applicationId,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(`/v1/auth/verify-mfa/webauthn`, {
      email,
      sessionId,
      assertionData,
      applicationId,
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
