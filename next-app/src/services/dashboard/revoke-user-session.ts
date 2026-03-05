import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  organizationId: string;
  userId: string;
  sessionId: string;
};

export async function revokeUserSessionApi(
  { organizationId, userId, sessionId }: Request,
  { accessToken }: IServiceOptions,
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete(
      `/v1/organizations/${organizationId}/users/${userId}/sessions/${sessionId}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      },
    );
    return [null];
  } catch (error: unknown) {
    return [error as APIError];
  }
}
