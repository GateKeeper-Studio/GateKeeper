// Server-side OAuth2/OIDC client against the Rust Identity Provider.
//
// Everything here runs only on the server: the client secret and the raw tokens
// never reach the browser. The authorize *redirect* points at the dashboard's
// hosted login UI (AUTH_UI_BASE_URL); the token/userinfo calls go straight to the
// IdP (IDP_BASE_URL) over a server→server channel.
import { env } from '$env/dynamic/private';

export function oauthConfig() {
  const cfg = {
    idpBaseUrl: env.IDP_BASE_URL,
    authUiBaseUrl: env.AUTH_UI_BASE_URL,
    tenantId: env.OAUTH_TENANT_ID,
    clientId: env.OAUTH_CLIENT_ID,
    clientSecret: env.OAUTH_CLIENT_SECRET,
    redirectUri: env.OAUTH_REDIRECT_URI,
    scope: env.OAUTH_SCOPE || 'openid profile email'
  };
  const missing = Object.entries(cfg)
    .filter(([k, v]) => k !== 'clientSecret' && !v)
    .map(([k]) => k);
  if (missing.length) throw new Error(`Missing OAuth env config: ${missing.join(', ')}`);
  return cfg;
}

// Build the URL the browser is redirected to in order to sign in. We hand all the
// OIDC parameters to the dashboard login page, which validates credentials and
// redirects back to our redirect_uri with ?code=...&state=...
export function buildAuthorizeUrl({ state, nonce, codeChallenge, mfa }) {
  const cfg = oauthConfig();
  const params = new URLSearchParams({
    redirect_uri: cfg.redirectUri,
    response_type: 'code',
    scope: cfg.scope,
    state,
    nonce,
    code_challenge: codeChallenge,
    code_challenge_method: 'S256'
  });
  if (mfa?.methods?.length) {
    params.set('mfa_methods', mfa.methods.join(','));
  }
  if (mfa?.defaultMethod) {
    params.set('mfa_default_method', mfa.defaultMethod);
  }
  return `${cfg.authUiBaseUrl}/auth/${cfg.clientId}/sign-in?${params.toString()}`;
}

// Decode a JWT payload without verifying the signature. Safe here because tokens
// are received over a trusted server→server TLS channel directly from the IdP;
// we only read claims (exp, nonce, profile) we already trust.
export function decodeJwtPayload(token) {
  try {
    const part = token.split('.')[1];
    if (!part) return null;
    return JSON.parse(Buffer.from(part, 'base64url').toString('utf8'));
  } catch {
    return null;
  }
}

async function postToken(form) {
  const cfg = oauthConfig();
  const res = await fetch(`${cfg.idpBaseUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams(form)
  });
  if (!res.ok) {
    const detail = await res.text().catch(() => '');
    throw new Error(`token endpoint ${res.status}: ${detail}`);
  }
  return res.json();
}

// Exchange a single-use authorization code for tokens (PKCE-protected).
export function exchangeCode({ code, codeVerifier }) {
  const cfg = oauthConfig();
  return postToken({
    tenant_id: cfg.tenantId,
    grant_type: 'authorization_code',
    client_id: cfg.clientId,
    client_secret: cfg.clientSecret ?? '',
    code,
    redirect_uri: cfg.redirectUri,
    code_verifier: codeVerifier
  });
}

// Rotate the refresh token and mint a fresh access token.
export function refreshTokens(refreshToken) {
  const cfg = oauthConfig();
  return postToken({
    tenant_id: cfg.tenantId,
    grant_type: 'refresh_token',
    client_id: cfg.clientId,
    client_secret: cfg.clientSecret ?? '',
    refresh_token: refreshToken
  });
}

// Fetch OIDC claims for an access token (optional; profile is also in the id_token).
export async function fetchUserInfo(accessToken) {
  const cfg = oauthConfig();
  const url = `${cfg.idpBaseUrl}/oauth/userinfo?tenant_id=${encodeURIComponent(cfg.tenantId)}`;
  const res = await fetch(url, { headers: { Authorization: `Bearer ${accessToken}` } });
  if (!res.ok) throw new Error(`userinfo endpoint ${res.status}`);
  return res.json();
}

// Distill a small, browser-safe profile from OIDC claims (id_token or userinfo).
export function profileFromClaims(claims) {
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

// Turn a raw token response into the session object we seal into the cookie.
// expires_at lets hooks.server.js decide when to silently refresh.
export function sessionFromTokenResponse(tokens) {
  const claims = decodeJwtPayload(tokens.id_token) ?? decodeJwtPayload(tokens.access_token);
  const expiresAt = typeof tokens.expires_in === 'number'
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
