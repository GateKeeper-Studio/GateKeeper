import { api } from "../base/gatekeeper-api";
import {
  APIError,
  IServiceOptions,
  ResultWithoutResponse,
} from "@/types/service-options";

type Request = {
  organizationId: string;
  applicationId: string;
  userId: string;
  sessionId: string;
};

export async function revokeUserSessionApi(
  { organizationId, applicationId, userId, sessionId }: Request,
  { accessToken }: IServiceOptions,
): Promise<ResultWithoutResponse<APIError>> {
  try {
    await api.delete(
      `/v1/organizations/${organizationId}/applications/${applicationId}/users/${userId}/sessions/${sessionId}`,
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
