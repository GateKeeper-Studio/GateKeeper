// Server-only helper for the IdP's authenticated self-service API (/me/*).
// Attaches the access token sealed inside the session cookie; the browser
// never sees the token — pages talk to our /api/* proxy routes instead.
import { error } from '@sveltejs/kit';
import { oauthConfig } from '$lib/server/oauth.js';

export async function idpFetch(locals, path, init = {}) {
  if (!locals.session?.accessToken) error(401, 'Not signed in');

  const res = await fetch(`${oauthConfig().idpBaseUrl}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${locals.session.accessToken}`,
      ...(init.body ? { 'Content-Type': 'application/json' } : {}),
      ...init.headers
    }
  });

  if (!res.ok) {
    const detail = await res.json().catch(() => null);
    const message =
      detail?.error ?? detail?.errors?.[0]?.message ?? `IdP request failed (${res.status})`;
    error(res.status, message);
  }

  if (res.status === 204) return null;
  return res.json();
}
