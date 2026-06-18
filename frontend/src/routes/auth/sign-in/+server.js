// GET /auth/sign-in?returnTo=/some/path
//
// Starts the Authorization Code + PKCE flow: mint the transient secrets, stash
// them in short-lived httpOnly cookies, then redirect the browser to the hosted
// login page. After login the user is sent back to `returnTo`.
import { redirect } from '@sveltejs/kit';
import { generatePkce } from '$lib/server/pkce.js';
import { buildAuthorizeUrl } from '$lib/server/oauth.js';
import { configureMfaMethods } from '$lib/server/mfa.js';
import { dev } from '$app/environment';

// Must outlive the backend's email-verification window (15 min, see
// VERIFICATION_CODE_TTL_MINUTES). A sign-up round-trip includes receiving an
// email and typing back a 6-digit code, so these transient secrets have to
// survive far longer than a plain login. 20 minutes covers the full TTL plus
// the redirect overhead; if they expire first the callback can no longer match
// the state and fails with "Invalid or missing OAuth state".
const TEMP_COOKIE = {
  httpOnly: true,
  sameSite: 'lax',
  secure: !dev,
  path: '/',
  maxAge: 1200
};

// Only allow returning to a local path, never to an attacker-controlled absolute
// URL (open-redirect defense). "//evil.com" is rejected because of the second char.
function safeReturnTo(value) {
  if (value && value.startsWith('/') && !value.startsWith('//')) return value;
  return '/';
}

function mfaConfigFromSearchParams(searchParams) {
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

export function GET({ url, cookies }) {
  const returnTo = safeReturnTo(url.searchParams.get('returnTo'));
  const mfa = mfaConfigFromSearchParams(url.searchParams);
  const pkce = generatePkce();

  cookies.set('gk_code_verifier', pkce.codeVerifier, TEMP_COOKIE);
  cookies.set('gk_state', pkce.state, TEMP_COOKIE);
  cookies.set('gk_nonce', pkce.nonce, TEMP_COOKIE);
  cookies.set('gk_return_to', returnTo, TEMP_COOKIE);

  redirect(
    302,
    buildAuthorizeUrl({
      state: pkce.state,
      nonce: pkce.nonce,
      codeChallenge: pkce.codeChallenge,
      mfa
    })
  );
}
