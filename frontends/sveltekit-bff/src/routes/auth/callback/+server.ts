// GET /auth/callback?code=...&state=...
//
// The login page redirects here with a single-use authorization code. We validate
// the CSRF state, exchange the code for tokens server-side, verify the OIDC nonce,
// seal the session into an httpOnly cookie, and bounce back to the page that asked
// for sign-in.
import { error, redirect } from '@sveltejs/kit';
import { gatekeeper, setSession } from '$lib/server/gatekeeper.js';
import type { Cookies } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

function clearTempCookies(cookies: Cookies): void {
  for (const name of ['gk_code_verifier', 'gk_state', 'gk_nonce', 'gk_return_to']) {
    cookies.delete(name, { path: '/' });
  }
}

export const GET: RequestHandler = async ({ url, cookies }) => {
  const client = gatekeeper();
  const code = url.searchParams.get('code');
  const state = url.searchParams.get('state');

  const expectedState = cookies.get('gk_state');
  const codeVerifier = cookies.get('gk_code_verifier');
  const expectedNonce = cookies.get('gk_nonce');
  const returnTo = cookies.get('gk_return_to') || '/';

  // CSRF: the state echoed back must match the one we issued.
  if (!code || !state || !expectedState || state !== expectedState) {
    clearTempCookies(cookies);
    error(400, 'Invalid or missing OAuth state');
  }
  if (!codeVerifier) {
    clearTempCookies(cookies);
    error(400, 'Missing PKCE verifier — restart the sign-in flow');
  }

  let tokens;
  try {
    tokens = await client.exchangeCode({ code, codeVerifier });
  } catch (e) {
    clearTempCookies(cookies);
    const message = e instanceof Error ? e.message : String(e);
    error(400, `Token exchange failed: ${message}`);
  }

  // OIDC replay defense: the id_token's nonce must match what we sent.
  if (tokens.id_token && expectedNonce) {
    const claims = client.decodeJwtPayload(tokens.id_token);
    if (claims?.nonce !== expectedNonce) {
      clearTempCookies(cookies);
      error(400, 'Nonce mismatch — possible replay attack');
    }
  }

  setSession(cookies, client.sessionFromTokenResponse(tokens));
  clearTempCookies(cookies);

  redirect(302, returnTo);
};
