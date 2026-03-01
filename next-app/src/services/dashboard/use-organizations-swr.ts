import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

export type Organization = {
  id: string;
  name: string;
  description: string;
  createdAt: Date;
};

type Response = Organization[];

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useOrganizationsSWR(options: IServiceOptions) {
  return useSWR("/v1/organizations", (url) => fetcher(url, options), {
    revalidateOnFocus: false,
    dedupingInterval: 60000 * 10, // 10 minutes
  });
}
