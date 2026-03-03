import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";
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
  refreshTokenTtlDays: number;
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
const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useApplicationByIdSWR(
  request: Request,
  options: IServiceOptions,
) {
  return useSWR(
    request?.organizationId
      ? `/v1/organizations/${request?.organizationId}/applications/${request?.applicationId}`
      : null,
    (url) => fetcher(url, options),
    {
      revalidateOnFocus: false,
      dedupingInterval: 60000 * 10, // 10 minutes
    },
  );
}
