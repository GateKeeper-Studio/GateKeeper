import { api } from "../base/gatekeeper-api";
import { APIError, IServiceOptions, Result } from "@/types/service-options";

export interface IApplication {
  id: string;
  name: string;
  description: string;
  badges: string[];
  createdAt: Date;
  updatedAt: Date;
  isActive: boolean;
  mfaEmailEnabled: boolean;
  canSelfSignUp: boolean;
  canSelfForgotPass: boolean;
  mfaAuthAppEnabled: boolean;
  mfaWebauthnEnabled: boolean;
  passwordHashingSecret: string;
  secrets: {
    id: string;
    name: string;
    value: string;
    expirationDate: Date;
  }[];
  users: {
    totalCount: number;
    data: {
      id: string;
      displayName: string;
      email: string;
      roles: {
        id: string;
        name: string;
        description: string;
      }[];
    }[];
  };
  roles: {
    totalCount: number;
    data: {
      id: string;
      name: string;
      description: string;
    }[];
  };
  oauthProviders: {
    id: string;
    name: string;
    redirectUri: string;
    clientId: string;
    clientSecret: string;
    isEnabled: boolean;
  }[];
}

type Request = {
  applicationId: string;
  organizationId?: string;
};

type Response = IApplication;

export async function getApplicationByIdService(
  { applicationId, organizationId }: Request,
  { accessToken }: IServiceOptions,
): Promise<Result<Response, APIError>> {
  try {
    const { data } = await api.get<Response>(
      `/v1/organizations/${organizationId}/applications/${applicationId}`,
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
