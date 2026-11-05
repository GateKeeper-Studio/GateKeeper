import { idpFetch } from './idp.js';
import type {
  GateKeeperSession,
  LocalsLike,
  ResolvedGateKeeperConfig,
  UpdateUserProfileInput,
  UserProfileView
} from './types.js';

interface UserProfileResponse {
  user_id: string;
  display_name: string | null;
  first_name: string;
  last_name: string;
  email: string;
}

function toUserProfileView(response: UserProfileResponse): UserProfileView {
  return {
    userId: response.user_id,
    displayName: response.display_name,
    firstName: response.first_name,
    lastName: response.last_name,
    email: response.email
  };
}

export async function updateMyProfile(
  cfg: ResolvedGateKeeperConfig,
  sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
  input: UpdateUserProfileInput
): Promise<UserProfileView> {
  const response = await idpFetch<UserProfileResponse>(cfg, sessionOrLocals, '/me/profile', {
    method: 'PUT',
    body: JSON.stringify({
      display_name: input.displayName ?? null,
      first_name: input.firstName,
      last_name: input.lastName
    })
  });

  return toUserProfileView(response);
}

export function sessionWithUpdatedProfile(
  session: GateKeeperSession,
  profile: UserProfileView
): GateKeeperSession {
  return {
    ...session,
    profile: {
      ...(session.profile ?? {}),
      sub: session.profile?.sub ?? profile.userId,
      email: profile.email,
      name: profile.displayName ?? `${profile.firstName} ${profile.lastName}`,
      givenName: profile.firstName,
      familyName: profile.lastName
    }
  };
}
