import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

type Request = {
  organizationId: string;
  applicationId: string;
  userId: string;
};

export type UserSession = {
  id: string;
  userId: string;
  expiresAt: string;
  createdAt: string;
  isActive: boolean;
};

type Response = {
  data: UserSession[];
};

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useUserSessionsSWR(request: Request, options: IServiceOptions) {
  return useSWR(
    request?.organizationId && request?.applicationId && request?.userId
      ? `/v1/organizations/${request.organizationId}/applications/${request.applicationId}/users/${request.userId}/sessions`
      : null,
    (url) => fetcher(url, options),
    {
      revalidateOnFocus: false,
    },
  );
}
