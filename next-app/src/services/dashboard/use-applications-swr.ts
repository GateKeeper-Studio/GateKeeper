import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

type Request = {
  tenantId?: string;
};

export type ApplicationsResponse = {
  id: string;
  name: string;
  description: string;
  createdAt: Date;
  updatedAt: Date | null;
  isActive: boolean;
  badges: string[];
}[];

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<ApplicationsResponse>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useApplicationsSWR(request: Request, options: IServiceOptions) {
  return useSWR(
    request?.tenantId
      ? `/v1/tenants/${request?.tenantId}/applications`
      : null,
    (url) => fetcher(url, options),
    {
      revalidateOnFocus: false,
      dedupingInterval: 60000 * 10, // 10 minutes
    },
  );
}
