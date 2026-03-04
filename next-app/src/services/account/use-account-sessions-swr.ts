import useSWR from "swr";

import { api } from "../base/gatekeeper-api";

import type { IServiceOptions } from "@/types/service-options";

export type AccountSession = {
  id: string;
  ipAddress: string;
  userAgent: string;
  location: string | null;
  createdAt: string;
  lastActiveAt: string;
  expiresAt: string;
};

type Response = {
  sessions: AccountSession[];
  total: number;
};

const fetcher = (url: string, options: IServiceOptions, stepUpToken: string) =>
  api
    .get<Response>(url, {
      headers: {
        Authorization: `Bearer ${options.accessToken}`,
        "X-Step-Up-Token": stepUpToken,
      },
    })
    .then((res) => res.data);

export function useAccountSessionsSWR(
  options: IServiceOptions,
  stepUpToken: string | null,
) {
  return useSWR(
    stepUpToken ? ["/v1/account/sessions", stepUpToken] : null,
    ([url, token]) => fetcher(url, options, token),
    {
      revalidateOnFocus: false,
    },
  );
}
