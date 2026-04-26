import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

export type Tenant = {
  id: string;
  name: string;
  description: string;
  createdAt: Date;
};

type Response = Tenant[];

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useTenantsSWR(options: IServiceOptions) {
  return useSWR("/v1/tenants", (url) => fetcher(url, options), {
    revalidateOnFocus: false,
    dedupingInterval: 60000 * 10, // 10 minutes
  });
}
