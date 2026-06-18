// Authenticated account-security page: requires a session and loads the
// user's MFA methods from the IdP for the initial render.
import { redirect } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/idp.js';

export async function load({ locals, url }) {
  if (!locals.session) {
    redirect(302, `/auth/sign-in?returnTo=${encodeURIComponent(url.pathname)}`);
  }
  const mfa = await idpFetch(locals, '/me/mfa/methods');
  return { user: locals.session.profile, mfa };
}
