// OAuth wire helpers for the public SPA flow. Mirrors the server contract:
// hosted-login authorize URL, form-encoded /oauth/token, bearer /oauth/userinfo.

import { base64UrlDecode } from './pkce.js';
import type {
  GateKeeperProfile,
  JwtClaims,
  PkceTransaction,
  ResolvedSpaConfig,
  StoredTokens,
  TokenResponse
} from './types.js';

export function trimTrailingSlash(value: string): string {
  return String(value).replace(/\/+$/, '');
}

export function buildAuthorizeUrl(
  cfg: ResolvedSpaConfig,
  transaction: PkceTransaction,
  codeChallenge: string,
  options: { scope?: string; loginHint?: string; extraParams?: Record<string, string> } = {}
): string {
  const requestExpiresAt = Math.floor(Date.now() / 1000) + cfg.authRequestTtlSeconds;
  const params = new URLSearchParams({
    redirect_uri: transaction.redirectUri,
    response_type: 'code',
    scope: options.scope ?? cfg.scope,
    state: transaction.state,
    nonce: transaction.nonce,
    code_challenge: codeChallenge,
    code_challenge_method: 'S256',
    request_expires_at: String(requestExpiresAt)
  });

  if (options.loginHint) params.set('login_hint', options.loginHint);
  for (const [key, value] of Object.entries(options.extraParams ?? {})) {
    params.set(key, value);
  }

  return `${cfg.authUiBaseUrl}/auth/${cfg.clientId}/sign-in?${params.toString()}`;
}

export function decodeJwtPayload(token: string): JwtClaims | null {
  try {
    const part = token.split('.')[1];
    if (!part) return null;
    return JSON.parse(base64UrlDecode(part)) as JwtClaims;
  } catch {
    return null;
  }
}

async function postToken(
  cfg: ResolvedSpaConfig,
  fields: Record<string, string>
): Promise<TokenResponse> {
  const body = new URLSearchParams({
    tenant_id: cfg.tenantId,
    client_id: cfg.clientId,
    ...fields
  });

  const res = await fetch(`${cfg.idpBaseUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body
  });

  if (!res.ok) {
    const detail = await res.text().catch(() => '');
    const err = new Error(`token endpoint ${res.status}: ${detail}`) as Error & {
      status?: number;
      oauthError?: string;
    };
    err.status = res.status;
    try {
      err.oauthError = (JSON.parse(detail) as { error?: string }).error;
    } catch {
      // Body was not JSON.
    }
    throw err;
  }

  return (await res.json()) as TokenResponse;
}

export function exchangeCode(
  cfg: ResolvedSpaConfig,
  input: { code: string; codeVerifier: string; redirectUri: string }
): Promise<TokenResponse> {
  // Public client: no client_secret is sent.
  return postToken(cfg, {
    grant_type: 'authorization_code',
    code: input.code,
    redirect_uri: input.redirectUri,
    code_verifier: input.codeVerifier
  });
}

export function refreshTokens(cfg: ResolvedSpaConfig, refreshToken: string): Promise<TokenResponse> {
  return postToken(cfg, {
    grant_type: 'refresh_token',
    refresh_token: refreshToken
  });
}

export async function fetchUserInfo(cfg: ResolvedSpaConfig, accessToken: string): Promise<JwtClaims> {
  const url = `${cfg.idpBaseUrl}/oauth/userinfo?tenant_id=${encodeURIComponent(cfg.tenantId)}`;
  const res = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
  if (!res.ok) throw new Error(`userinfo endpoint ${res.status}`);
  return (await res.json()) as JwtClaims;
}

export function profileFromClaims(claims: JwtClaims | null | undefined): GateKeeperProfile | null {
  if (!claims) return null;
  return {
    sub: claims.sub,
    email: claims.email,
    emailVerified: claims.email_verified,
    name: claims.name,
    givenName: claims.given_name,
    familyName: claims.family_name,
    picture: claims.picture
  };
}

export function tokensFromResponse(response: TokenResponse): StoredTokens {
  const accessClaims = decodeJwtPayload(response.access_token);
  const idClaims = response.id_token ? decodeJwtPayload(response.id_token) : null;
  const claims = idClaims ?? accessClaims;
  const expiresAt =
    typeof response.expires_in === 'number'
      ? Math.floor(Date.now() / 1000) + response.expires_in
      : accessClaims?.exp ?? null;

  return {
    accessToken: response.access_token,
    idToken: response.id_token ?? null,
    refreshToken: response.refresh_token ?? null,
    tokenType: response.token_type,
    scope: response.scope,
    expiresAt,
    claims
  };
}
