// Authenticated account-security page: requires a session and loads the
// user's MFA methods plus their recent sign-in activity from the IdP for the
// initial render.
import { redirect } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { PageServerLoad } from './$types';

interface LoginEventsResponse {
  items?: unknown[];
}

export const load: PageServerLoad = async ({ locals, url }) => {
  if (!locals.session) {
    redirect(302, `/auth/sign-in?returnTo=${encodeURIComponent(url.pathname)}`);
  }
  // Recent sign-in activity is non-critical: a failure here (e.g. the endpoint is
  // unavailable) must not break the security page, so swallow it and fall back to
  // an empty list.
  const [mfa, activity] = await Promise.all([
    idpFetch(locals, '/me/mfa/methods'),
    idpFetch<LoginEventsResponse>(locals, '/me/security/login-events?page_size=10').catch(() => null)
  ]);
  return {
    user: locals.session.profile,
    mfa,
    loginEvents: activity?.items ?? []
  };
};
