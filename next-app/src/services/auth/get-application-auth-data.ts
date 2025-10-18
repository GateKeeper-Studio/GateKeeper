import { api } from "../base/gatekeeper-api";
import { APIError, Result } from "@/types/service-options";

type Request = {
  applicationId: string;
};

type Response = ApplicationAuthData;

export type ApplicationAuthData = {
  id: string;
  name: string;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
  oauthProviders: {
    id: string;
    name: string;
  }[];
};

export async function getApplicationAuthDataService({
  applicationId,
}: Request): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(
      `/v1/auth/application/${applicationId}/auth-data`
    );

    await new Promise((resolve) => setTimeout(resolve, 0));

    return [data, null];
  } catch (error: unknown) {
    return [null, error as APIError];
  }
}
