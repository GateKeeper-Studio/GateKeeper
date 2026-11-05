// GET /auth/sign-in?returnTo=/some/path&login_hint=user@example.com
//
// Starts the Authorization Code + PKCE flow: mint the transient secrets, stash
// them in short-lived httpOnly cookies, then redirect the browser to the hosted
// login page. After login the user is sent back to `returnTo`.
//
// This is also our OIDC `initiate_login_uri` — register it on the OAuth client so the
// hosted login page can send users back here to restart when the OAuth state it was
// handed has expired. Nothing extra is needed for that: every GET mints fresh secrets
// and overwrites the cookies, so a restart is just another visit.
import { redirect } from '@sveltejs/kit';
import { configureMfaMethods, gatekeeper } from '$lib/server/gatekeeper.js';
import { dev } from '$app/environment';
import type { RequestHandler } from './$types';

// Max-Age comes from the same configured TTL that feeds the `request_expires_at` hint.
function tempCookieOptions(client: ReturnType<typeof gatekeeper>) {
  return {
    httpOnly: true,
    sameSite: 'lax' as const,
    secure: !dev,
    path: '/',
    maxAge: client.authRequestTtlSeconds
  };
}

// Only allow returning to a local path, never to an attacker-controlled absolute
// URL (open-redirect defense). "//evil.com" is rejected because of the second char.
function safeReturnTo(value: string | null): string {
  if (value && value.startsWith('/') && !value.startsWith('//')) return value;
  return '/';
}

function mfaConfigFromSearchParams(searchParams: URLSearchParams) {
  const methods = searchParams
    .get('mfa_methods')
    ?.split(',')
    .map((method) => method.trim())
    .filter(Boolean);
  if (!methods?.length) return undefined;

  // Optional: the user's server-side preference (set on /account/security)
  // wins anyway; a hint here only reorders the login UI.
  const defaultMethod = searchParams.get('mfa_default_method') || undefined;

  return configureMfaMethods({
    methods,
    defaultMethod
  });
}

export const GET: RequestHandler = ({ url, cookies }) => {
  const client = gatekeeper();
  const returnTo = safeReturnTo(url.searchParams.get('returnTo'));
  const mfa = mfaConfigFromSearchParams(url.searchParams);
  const pkce = client.generatePkce();
  const tempCookie = tempCookieOptions(client);

  cookies.set('gk_code_verifier', pkce.codeVerifier, tempCookie);
  cookies.set('gk_state', pkce.state, tempCookie);
  cookies.set('gk_nonce', pkce.nonce, tempCookie);
  cookies.set('gk_return_to', returnTo, tempCookie);

  redirect(
    302,
    client.buildAuthorizeUrl({
      state: pkce.state,
      nonce: pkce.nonce,
      codeChallenge: pkce.codeChallenge,
      mfa,
      // Set when the login page bounced the user back here to restart an expired
      // request; it just carries their email across so they don't type it twice.
      loginHint: url.searchParams.get('login_hint') ?? undefined
    })
  );
};
