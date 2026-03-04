import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
  token: string;
  email: string;
  codeChallengeMethod: string;
  responseType: string;
  scope: string;
  state: string;
  codeChallenge: string;
  redirectUri: string;
  nonce: string;
};

type Response = {
  authorizationCode: string;
  state: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  responseType: string;
  redirectUri: string;
};

export async function confirmEmailApi({
  applicationId,
  token,
  email,
  codeChallengeMethod,
  responseType,
  scope,
  state,
  codeChallenge,
  redirectUri,
  nonce,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.post<Response>(`/v1/auth/confirm-email`, {
      applicationId,
      token,
      email,
      codeChallengeMethod,
      responseType,
      scope,
      state,
      codeChallenge,
      redirectUri,
      ...(nonce ? { nonce } : {}),
    });

    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
