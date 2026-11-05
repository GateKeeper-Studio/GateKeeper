// Framework-agnostic Backend-for-Frontend runtime. Encapsulates the confidential
// Auth Code + PKCE flow over a generic `CookiesLike` jar so hosts (Next.js, Remix)
// only wire their own cookie adapter and routing. Includes refresh coalescing so
// the same refresh token is never presented twice concurrently (the IdP treats
// that as reuse and revokes the whole token family).

import { createGateKeeperClient } from './client.js';
import type {
  BffOptions,
  BrowserSafeSession,
  CompleteSignInInput,
  CompleteSignInResult,
  CookieOptions,
  CookiesLike,
  GateKeeperBff,
  GateKeeperConfig,
  GateKeeperSession,
  StartSignInInput,
  StartSignInResult
} from './types.js';

const DEFAULT_REFRESH_SKEW_SECONDS = 120;
const DEFAULT_TRANSACTION_TTL_SECONDS = 600;

function isDefinitiveAuthFailure(err: unknown): boolean {
  const failure = err as { oauthError?: string; status?: number } | null | undefined;
  return (
    failure?.oauthError === 'invalid_grant' ||
    (typeof failure?.status === 'number' && failure.status >= 400 && failure.status < 500)
  );
}

export function createGateKeeperBff(
  config: GateKeeperConfig,
  options: BffOptions = {}
): GateKeeperBff {
  const client = createGateKeeperClient(config);
  const refreshSkewSeconds = options.refreshSkewSeconds ?? DEFAULT_REFRESH_SKEW_SECONDS;
  const prefix = options.transactionCookiePrefix || 'gk';
  const secure = client.config.secureCookies;

  const names = {
    codeVerifier: `${prefix}_code_verifier`,
    state: `${prefix}_state`,
    nonce: `${prefix}_nonce`,
    returnTo: `${prefix}_return_to`
  };

  const transactionCookie: CookieOptions = {
    httpOnly: true,
    sameSite: 'lax',
    secure,
    path: '/',
    maxAge: DEFAULT_TRANSACTION_TTL_SECONDS
  };

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

  function startSignIn(cookies: CookiesLike, input: StartSignInInput = {}): StartSignInResult {
    const pkce = client.generatePkce();
    cookies.set(names.codeVerifier, pkce.codeVerifier, transactionCookie);
    cookies.set(names.state, pkce.state, transactionCookie);
    cookies.set(names.nonce, pkce.nonce, transactionCookie);
    cookies.set(names.returnTo, input.returnTo || '/', transactionCookie);

    const authorizeUrl = client.buildAuthorizeUrl({
      state: pkce.state,
      nonce: pkce.nonce,
      codeChallenge: pkce.codeChallenge,
      mfa: input.mfa,
      loginHint: input.loginHint
    });

    return { authorizeUrl };
  }

  function clearTransaction(cookies: CookiesLike): void {
    cookies.delete(names.codeVerifier, { path: '/' });
    cookies.delete(names.state, { path: '/' });
    cookies.delete(names.nonce, { path: '/' });
    cookies.delete(names.returnTo, { path: '/' });
  }

  async function completeSignIn(
    cookies: CookiesLike,
    input: CompleteSignInInput
  ): Promise<CompleteSignInResult> {
    const expectedState = cookies.get(names.state);
    const codeVerifier = cookies.get(names.codeVerifier);
    const expectedNonce = cookies.get(names.nonce);
    const returnTo = cookies.get(names.returnTo) || '/';

    if (!expectedState || !codeVerifier || input.state !== expectedState) {
      clearTransaction(cookies);
      throw new Error('OAuth state mismatch (possible CSRF). Start sign-in again.');
    }

    const tokens = await client.exchangeCode({ code: input.code, codeVerifier });

    if (expectedNonce && tokens.id_token) {
      const claims = client.decodeJwtPayload(tokens.id_token);
      if (claims?.nonce && claims.nonce !== expectedNonce) {
        clearTransaction(cookies);
        throw new Error('OAuth nonce mismatch (possible replay).');
      }
    }

    const session = client.sessionFromTokenResponse(tokens);
    client.setSession(cookies, session);
    clearTransaction(cookies);
    return { session, returnTo };
  }

  async function loadSession(cookies: CookiesLike): Promise<GateKeeperSession | null> {
    const session = client.readSession(cookies);
    if (!session) return null;

    const now = Math.floor(Date.now() / 1000);
    const expiringSoon = !session.expiresAt || session.expiresAt - now <= refreshSkewSeconds;
    if (!expiringSoon || !session.refreshToken) return session;

    try {
      const refreshed = await refreshOnce(session.refreshToken, session);
      client.setSession(cookies, refreshed);
      return refreshed;
    } catch (err) {
      if (isDefinitiveAuthFailure(err)) {
        client.clearSession(cookies);
        return null;
      }
      // Transient failure — keep the current session and retry on a later request.
      return session;
    }
  }

  function signOut(cookies: CookiesLike): void {
    client.clearSession(cookies);
    clearTransaction(cookies);
  }

  function getBrowserSession(session: GateKeeperSession | null | undefined): BrowserSafeSession {
    return {
      authenticated: Boolean(session),
      user: session?.profile ?? null,
      expiresAt: session?.expiresAt ?? null
    };
  }

  return {
    ...client,
    startSignIn,
    completeSignIn,
    loadSession,
    signOut,
    getBrowserSession
  };
}
