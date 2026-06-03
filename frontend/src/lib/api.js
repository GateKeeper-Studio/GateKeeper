// Thin fetch wrappers around the Rust provider's OAuth/OIDC endpoints.
//
// No tokens are ever placed in the URL except the one-time authorization code on
// the redirect (per spec). All token requests go over POST bodies / headers.

import { issuerBase } from './config.js';

async function asJsonOrThrow(res, context) {
  let body = null;
  const text = await res.text();
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    body = { raw: text };
  }
  if (!res.ok) {
    const detail = body && (body.error || body.errors) ? JSON.stringify(body) : text;
    throw new Error(`${context} failed (${res.status}): ${detail}`);
  }
  return body;
}

// Step 1: validate the authorization request. The provider returns JSON (it does
// not host a login page), including the resolved client name + intersected scopes.
export async function authorizeRequest(cfg, pkce) {
  const params = new URLSearchParams({
    tenant_id: cfg.tenantId,
    client_id: cfg.clientId,
    redirect_uri: cfg.redirectUri,
    response_type: 'code',
    scope: cfg.scope,
    state: pkce.state,
    code_challenge: pkce.codeChallenge,
    code_challenge_method: pkce.codeChallengeMethod,
    nonce: pkce.nonce
  });
  const res = await fetch(`${issuerBase(cfg)}/oauth/authorize?${params}`, {
    method: 'GET',
    headers: { Accept: 'application/json' }
  });
  return asJsonOrThrow(res, 'authorize');
}

// Step 2: record consent and obtain the single-use authorization code.
//
// We send `Accept: application/json` so the provider returns the code in a JSON
// body instead of a 302. A SPA cannot follow a cross-origin 302: fetch would
// auto-follow it into a GET on redirect_uri's origin (the static file server),
// which sends no CORS headers and gets blocked. Reading the code from JSON lets
// us drive the rest of the flow ourselves.
export async function consent(cfg, pkce, validated) {
  const body = {
    tenant_id: cfg.tenantId,
    client_id: cfg.clientId,
    user_id: cfg.userId,
    redirect_uri: cfg.redirectUri,
    scopes: validated.scopes,
    state: pkce.state,
    code_challenge: pkce.codeChallenge,
    code_challenge_method: pkce.codeChallengeMethod,
    nonce: pkce.nonce,
    user_approved: true
  };

  const res = await fetch(`${issuerBase(cfg)}/oauth/authorize/consent`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json'
    },
    body: JSON.stringify(body)
  });

  const data = await asJsonOrThrow(res, 'consent');
  if (!data.code) throw new Error('consent did not return an authorization code');
  return { code: data.code, state: data.state };
}

// Step 3: exchange the code for tokens. Per spec this is form-urlencoded.
export async function exchangeCode(cfg, code, codeVerifier) {
  const form = new URLSearchParams({
    tenant_id: cfg.tenantId,
    grant_type: 'authorization_code',
    client_id: cfg.clientId,
    code,
    redirect_uri: cfg.redirectUri,
    code_verifier: codeVerifier
  });
  if (cfg.clientSecret) form.set('client_secret', cfg.clientSecret);

  const res = await fetch(`${issuerBase(cfg)}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: form
  });
  return asJsonOrThrow(res, 'token exchange');
}

// Rotate a refresh token for a fresh access (+ id) token.
export async function refresh(cfg, refreshToken) {
  const form = new URLSearchParams({
    tenant_id: cfg.tenantId,
    grant_type: 'refresh_token',
    client_id: cfg.clientId,
    refresh_token: refreshToken
  });
  if (cfg.clientSecret) form.set('client_secret', cfg.clientSecret);

  const res = await fetch(`${issuerBase(cfg)}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: form
  });
  return asJsonOrThrow(res, 'token refresh');
}

// Call the OIDC UserInfo endpoint with the access token as a Bearer credential.
export async function userInfo(cfg, accessToken) {
  const params = new URLSearchParams({ tenant_id: cfg.tenantId });
  const res = await fetch(`${issuerBase(cfg)}/oauth/userinfo?${params}`, {
    method: 'GET',
    headers: { Authorization: `Bearer ${accessToken}` }
  });
  return asJsonOrThrow(res, 'userinfo');
}

export async function jwks(cfg) {
  const params = new URLSearchParams({ tenant_id: cfg.tenantId });
  const res = await fetch(`${issuerBase(cfg)}/.well-known/jwks.json?${params}`);
  return asJsonOrThrow(res, 'jwks');
}

export async function discovery(cfg) {
  const res = await fetch(`${issuerBase(cfg)}/.well-known/openid-configuration`);
  return asJsonOrThrow(res, 'discovery');
}
