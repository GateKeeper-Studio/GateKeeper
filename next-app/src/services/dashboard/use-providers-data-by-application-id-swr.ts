import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

type Request = {
  applicationId: string;
  tenantId?: string;
};

type Response = {
  id: string;
  name: string;
  redirectUri: string;
  clientId: string;
  clientSecret: string;
  isEnabled: boolean;
}[];

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useProvidersDataByApplicationIdSWR(
  request: Request,
  options: IServiceOptions,
) {
  return useSWR(
    request?.tenantId && request?.applicationId
      ? `/v1/tenants/${request?.tenantId}/applications/${request?.applicationId}/oauth-provider`
      : null,
    (url) => fetcher(url, options),
    {
      revalidateOnFocus: false,
      keepPreviousData: true,
      dedupingInterval: 60000 * 10, // 10 minutes
    },
  );
}
