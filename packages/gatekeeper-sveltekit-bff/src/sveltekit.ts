import { createGateKeeperClient } from './client.js';
import type {
  CookiesLike,
  BrowserSafeSession,
  GateKeeperConfig,
  GateKeeperProfile,
  GateKeeperSession,
  LocalsLike,
  SvelteKitBffOptions,
  SvelteKitEventLike,
  SvelteKitGateKeeperClient,
  SvelteKitHandleInput,
  SvelteKitSecurityHeaders,
  UpdateUserProfileInput,
  UserProfileView
} from './types.js';

const DEFAULT_REFRESH_SKEW_SECONDS = 120;

const DEFAULT_SECURITY_HEADERS: Required<SvelteKitSecurityHeaders> = {
  frameOptions: 'DENY',
  contentTypeOptions: 'nosniff',
  referrerPolicy: 'no-referrer'
};

export class GateKeeperSessionRequiredError extends Error {
  constructor(message = 'Not signed in') {
    super(message);
    this.name = 'GateKeeperSessionRequiredError';
  }
}

function cookiesLike(event: SvelteKitEventLike): CookiesLike {
  return event.cookies as CookiesLike;
}

function resolveSessionSource(
  source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
): GateKeeperSession | null {
  if (!source) return null;
  if ('accessToken' in source) return source;
  if ('locals' in source) return source.locals.session ?? null;
  return source.session ?? null;
}

function isDefinitiveAuthFailure(err: unknown): boolean {
  const failure = err as { oauthError?: string; status?: number } | null | undefined;
  return (
    failure?.oauthError === 'invalid_grant' ||
    (typeof failure?.status === 'number' && failure.status >= 400 && failure.status < 500)
  );
}

function applySecurityHeaders(response: Response, headers: SvelteKitSecurityHeaders | false): void {
  if (headers === false) return;

  const resolved = { ...DEFAULT_SECURITY_HEADERS, ...headers };
  if (resolved.frameOptions) response.headers.set('X-Frame-Options', resolved.frameOptions);
  if (resolved.contentTypeOptions) {
    response.headers.set('X-Content-Type-Options', resolved.contentTypeOptions);
  }
  if (resolved.referrerPolicy) response.headers.set('Referrer-Policy', resolved.referrerPolicy);
}

export function createSvelteKitGateKeeperClient(
  config: GateKeeperConfig,
  options: SvelteKitBffOptions = {}
): SvelteKitGateKeeperClient {
  const client = createGateKeeperClient(config);
  const refreshSkewSeconds = options.refreshSkewSeconds ?? DEFAULT_REFRESH_SKEW_SECONDS;
  const securityHeaders = options.securityHeaders ?? DEFAULT_SECURITY_HEADERS;
  const inflightRefreshes = new Map<string, Promise<GateKeeperSession>>();

  function refreshOnce(refreshToken: string, previous: GateKeeperSession): Promise<GateKeeperSession> {
    const existing = inflightRefreshes.get(refreshToken);
    if (existing) return existing;

    const pending = (async () => {
      const tokens = await client.refreshTokens(refreshToken);
      const refreshed = client.sessionFromTokenResponse(tokens);
      if (!refreshed.profile) refreshed.profile = previous.profile;
      return refreshed;
    })();

    pending.finally(() => inflightRefreshes.delete(refreshToken));
    inflightRefreshes.set(refreshToken, pending);
    return pending;
  }

  async function refreshSession(event: SvelteKitEventLike): Promise<GateKeeperSession | null> {
    const session = client.readSession(cookiesLike(event));
    if (!session) return null;

    const now = Math.floor(Date.now() / 1000);
    const expiringSoon = !session.expiresAt || session.expiresAt - now <= refreshSkewSeconds;
    if (!expiringSoon || !session.refreshToken) return session;

    try {
      const refreshed = await refreshOnce(session.refreshToken, session);
      client.setSession(cookiesLike(event), refreshed);
      return refreshed;
    } catch (err) {
      if (isDefinitiveAuthFailure(err)) {
        client.clearSession(cookiesLike(event));
        return null;
      }
      return session;
    }
  }

  async function loadSession(event: SvelteKitEventLike): Promise<GateKeeperSession | null> {
    const session = await refreshSession(event);
    event.locals.session = session;
    return session;
  }

  async function handle({ event, resolve }: SvelteKitHandleInput): Promise<Response> {
    await loadSession(event);
    const response = await resolve(event);
    applySecurityHeaders(response, securityHeaders);
    return response;
  }

  function getSession(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): GateKeeperSession | null {
    return resolveSessionSource(source);
  }

  function requireSession(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): GateKeeperSession {
    const session = getSession(source);
    if (!session) throw new GateKeeperSessionRequiredError();
    return session;
  }

  function getUser(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): GateKeeperProfile | null {
    return getSession(source)?.profile ?? null;
  }

  function getAccessToken(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): string | null {
    return getSession(source)?.accessToken ?? null;
  }

  function getIdToken(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): string | null {
    return getSession(source)?.idToken ?? null;
  }

  function getRefreshToken(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): string | null {
    return getSession(source)?.refreshToken ?? null;
  }

  function getBrowserSession(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): BrowserSafeSession {
    const session = getSession(source);
    return {
      authenticated: Boolean(session),
      user: session?.profile ?? null,
      expiresAt: session?.expiresAt ?? null
    };
  }

  async function updateMyProfileForEvent(
    event: SvelteKitEventLike,
    input: UpdateUserProfileInput
  ): Promise<UserProfileView> {
    const profile = await client.updateMyProfile(event.locals, input);
    const session = event.locals.session;
    if (session) {
      const updated = client.sessionWithUpdatedProfile(session, profile);
      event.locals.session = updated;
      client.setSession(cookiesLike(event), updated);
    }
    return profile;
  }

  return {
    ...client,
    loadSession,
    refreshSession,
    handle,
    getSession,
    requireSession,
    getUser,
    getAccessToken,
    getIdToken,
    getRefreshToken,
    getBrowserSession,
    updateMyProfileForEvent
  };
}
