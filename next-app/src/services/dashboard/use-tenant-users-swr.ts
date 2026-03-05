import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

type Request = {
  organizationId: string;
  page?: number;
  pageSize?: number;
};

type Response = {
  totalCount: number;
  page: number;
  pageSize: number;
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

const fetcher = (url: string, options: IServiceOptions) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
      },
    })
    .then((res) => res.data);

export function useTenantUsersSWR(
  request: Request,
  options: IServiceOptions,
) {
  const page = request?.page ?? 1;
  const pageSize = request?.pageSize ?? 10;

  return useSWR(
    request?.organizationId
      ? `/v1/organizations/${request?.organizationId}/users?page=${page}&pageSize=${pageSize}`
      : null,
    (url) => fetcher(url, options),
    {
      revalidateOnFocus: false,
      keepPreviousData: true,
      dedupingInterval: 60000 * 10, // 10 minutes
    },
  );
}
