import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  email: string;
  sessionCode: string;
  applicationId: string;
  redirectUri: string;
  codeChallengeMethod: string;
  responseType: string;
  scope: string;
  state: string;
  codeChallenge: string;
  nonce?: string;
  mfaId?: string;
};

type Response = {
  authorizationCode: string;
  state: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  responseType: string;
  redirectUri: string;
};

export async function authorizeApi({
  email,
  sessionCode,
  applicationId,
  redirectUri,
  codeChallengeMethod,
  responseType,
  scope,
  state,
  codeChallenge,
  nonce,
  mfaId,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(`/v1/auth/authorize`, {
      email,
      sessionCode,
      applicationId,
      redirectUri,
      codeChallengeMethod,
      responseType,
      scope,
      codeChallenge,
      state,
      mfaId,
      nonce,
    });
    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
