import { dev } from '$app/environment';
import { env } from '$env/dynamic/private';
import { error } from '@sveltejs/kit';
import type { Cookies } from '@sveltejs/kit';
import {
  GateKeeperHttpError,
  configureMfaMethods,
  createSvelteKitGateKeeperClient,
  mfaMethods
} from '@gatekeeper/sveltekit-bff';
import type {
  CookiesLike,
  GateKeeperConfig,
  GateKeeperSession,
  LocalsLike,
  SvelteKitGateKeeperClient,
  UpdateUserProfileInput,
  UserProfileView
} from '@gatekeeper/sveltekit-bff';

export { configureMfaMethods, mfaMethods };

export function gatekeeperConfig(): GateKeeperConfig {
  return {
    idpBaseUrl: env.IDP_BASE_URL,
    authUiBaseUrl: env.AUTH_UI_BASE_URL,
    tenantId: env.OAUTH_TENANT_ID,
    clientId: env.OAUTH_CLIENT_ID,
    clientSecret: env.OAUTH_CLIENT_SECRET,
    redirectUri: env.OAUTH_REDIRECT_URI,
    scope: env.OAUTH_SCOPE || 'openid profile email',
    sessionSecret: env.SESSION_SECRET,
    secureCookies: !dev
  };
}

let gatekeeperClient: SvelteKitGateKeeperClient | null = null;

export function gatekeeper(): SvelteKitGateKeeperClient {
  gatekeeperClient ??= createSvelteKitGateKeeperClient(gatekeeperConfig());
  return gatekeeperClient;
}

function cookiesLike(cookies: Cookies): CookiesLike {
  return cookies as unknown as CookiesLike;
}

export function readSession(cookies: Cookies): GateKeeperSession | null {
  return gatekeeper().readSession(cookiesLike(cookies));
}

export function setSession(cookies: Cookies, session: GateKeeperSession): void {
  gatekeeper().setSession(cookiesLike(cookies), session);
}

export function clearSession(cookies: Cookies): void {
  gatekeeper().clearSession(cookiesLike(cookies));
}

export function mapGateKeeperError(err: unknown): never {
  if (err instanceof GateKeeperHttpError) {
    error(err.status, err.message);
  }
  throw err;
}

export async function idpFetch<T = unknown>(
  locals: LocalsLike,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  try {
    return await gatekeeper().idpFetch<T>(locals, path, init);
  } catch (err) {
    mapGateKeeperError(err);
  }
}

export async function updateMyProfile(
  locals: App.Locals,
  cookies: Cookies,
  input: UpdateUserProfileInput
): Promise<UserProfileView> {
  try {
    return await gatekeeper().updateMyProfileForEvent({ locals, cookies }, input);
  } catch (err) {
    mapGateKeeperError(err);
  }
}
