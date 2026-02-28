import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  userId: string;
  applicationId: string;
  sessionId: string;
  credentialData: unknown;
};

export async function verifyWebAuthnRegistrationApi({
  userId,
  applicationId,
  sessionId,
  credentialData,
}: Request): Promise<Result<null, APIError>> {
  try {
    await api.post(`/v1/auth/webauthn/verify-registration`, {
      userId,
      applicationId,
      sessionId,
      credentialData,
    });
    return [null, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
