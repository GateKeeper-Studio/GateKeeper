// Runs on every request. Decrypts the session cookie into event.locals.session,
// transparently refreshes the access token shortly before it expires (no client
// timers needed), and stamps a few hardening headers on every response.
import { readSession, setSession, clearSession } from '$lib/server/session.js';
import { refreshTokens, sessionFromTokenResponse } from '$lib/server/oauth.js';

// Refresh when fewer than this many seconds remain on the access token.
const REFRESH_SKEW_SECONDS = 120;

async function loadSession(event) {
  const session = readSession(event.cookies);
  if (!session) return null;

  const now = Math.floor(Date.now() / 1000);
  const expiringSoon = !session.expiresAt || session.expiresAt - now <= REFRESH_SKEW_SECONDS;
  if (!expiringSoon || !session.refreshToken) return session;

  // Silent, server-side refresh with refresh-token rotation.
  try {
    const tokens = await refreshTokens(session.refreshToken);
    const refreshed = sessionFromTokenResponse(tokens);
    // A refresh response may omit the id_token; keep the known profile in that case.
    if (!refreshed.profile) refreshed.profile = session.profile;
    setSession(event.cookies, refreshed);
    return refreshed;
  } catch {
    clearSession(event.cookies);
    return null;
  }
}

export async function handle({ event, resolve }) {
  event.locals.session = await loadSession(event);

  const response = await resolve(event);
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('Referrer-Policy', 'no-referrer');
  return response;
}
