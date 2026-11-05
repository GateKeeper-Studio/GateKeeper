import type {
  AuthorizeUrlInput,
  GateKeeperConfig,
  GateKeeperProfile,
  GateKeeperSession,
  JwtClaims,
  ResolvedGateKeeperConfig,
  TokenResponse
} from './types.js';

export const DEFAULT_AUTH_REQUEST_TTL_SECONDS = 1200;

export function validateConfig(config: GateKeeperConfig): ResolvedGateKeeperConfig {
  const cfg: ResolvedGateKeeperConfig = {
    idpBaseUrl: config?.idpBaseUrl ?? '',
    authUiBaseUrl: config?.authUiBaseUrl ?? '',
    tenantId: config?.tenantId ?? '',
    clientId: config?.clientId ?? '',
    clientSecret: config?.clientSecret,
    redirectUri: config?.redirectUri ?? '',
    scope: config?.scope || 'openid profile email',
    sessionSecret: config?.sessionSecret ?? '',
    secureCookies: Boolean(config?.secureCookies),
    sessionCookieName: config?.sessionCookieName || 'gk_session',
    authRequestTtlSeconds: config?.authRequestTtlSeconds ?? DEFAULT_AUTH_REQUEST_TTL_SECONDS
  };

  const missing = Object.entries(cfg)
    .filter(([key, value]) => !['clientSecret', 'secureCookies'].includes(key) && !value)
    .map(([key]) => key);

  if (missing.length) {
    throw new Error(`Missing GateKeeper client config: ${missing.join(', ')}`);
  }

  return cfg;
}

export function buildAuthorizeUrl(cfg: ResolvedGateKeeperConfig, input: AuthorizeUrlInput): string {
  const params = new URLSearchParams({
    redirect_uri: cfg.redirectUri,
    response_type: 'code',
    scope: cfg.scope,
    state: input.state,
    nonce: input.nonce,
    code_challenge: input.codeChallenge,
    code_challenge_method: 'S256',
    request_expires_at: String(Math.floor(Date.now() / 1000) + cfg.authRequestTtlSeconds)
  });

  if (input.mfa?.methods?.length) {
    params.set('mfa_methods', input.mfa.methods.join(','));
  }
  if (input.mfa?.defaultMethod) {
    params.set('mfa_default_method', input.mfa.defaultMethod);
  }
  if (input.loginHint) {
    params.set('login_hint', input.loginHint);
  }

  return `${cfg.authUiBaseUrl}/auth/${cfg.clientId}/sign-in?${params.toString()}`;
}

export function decodeJwtPayload(token: string): JwtClaims | null {
  try {
    const part = token.split('.')[1];
    if (!part) return null;
    return JSON.parse(Buffer.from(part, 'base64url').toString('utf8')) as JwtClaims;
  } catch {
    return null;
  }
}

export async function postToken(
  cfg: ResolvedGateKeeperConfig,
  form: Record<string, string>
): Promise<TokenResponse> {
  const res = await fetch(`${cfg.idpBaseUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams(form)
  });

  if (!res.ok) {
    const detail = await res.text().catch(() => '');
    const err = new Error(`token endpoint ${res.status}: ${detail}`) as Error & {
      status?: number;
      oauthError?: string;
    };
    err.status = res.status;
    try {
      const parsed = JSON.parse(detail) as { error?: string };
      err.oauthError = parsed.error;
    } catch {
      // Body was not JSON.
    }
    throw err;
  }

  return (await res.json()) as TokenResponse;
}

export function exchangeCode(
  cfg: ResolvedGateKeeperConfig,
  { code, codeVerifier }: { code: string; codeVerifier: string }
): Promise<TokenResponse> {
  return postToken(cfg, {
    tenant_id: cfg.tenantId,
    grant_type: 'authorization_code',
    client_id: cfg.clientId,
    client_secret: cfg.clientSecret ?? '',
    code,
    redirect_uri: cfg.redirectUri,
    code_verifier: codeVerifier
  });
}

export function refreshTokens(
  cfg: ResolvedGateKeeperConfig,
  refreshToken: string
): Promise<TokenResponse> {
  return postToken(cfg, {
    tenant_id: cfg.tenantId,
    grant_type: 'refresh_token',
    client_id: cfg.clientId,
    client_secret: cfg.clientSecret ?? '',
    refresh_token: refreshToken
  });
}

export async function fetchUserInfo(
  cfg: ResolvedGateKeeperConfig,
  accessToken: string
): Promise<JwtClaims> {
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

export function sessionFromTokenResponse(tokens: TokenResponse): GateKeeperSession {
  const claims = decodeJwtPayload(tokens.id_token ?? '') ?? decodeJwtPayload(tokens.access_token);
  const expiresAt =
    typeof tokens.expires_in === 'number'
      ? Math.floor(Date.now() / 1000) + tokens.expires_in
      : decodeJwtPayload(tokens.access_token)?.exp ?? 0;

  return {
    profile: profileFromClaims(claims),
    accessToken: tokens.access_token,
    idToken: tokens.id_token,
    refreshToken: tokens.refresh_token,
    scope: tokens.scope,
    expiresAt
  };
}
