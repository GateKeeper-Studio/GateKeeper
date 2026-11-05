// Example route showing how to build a custom MFA UI from the package headless API.
import { redirect } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
  if (!locals.session) {
    redirect(302, `/auth/sign-in?returnTo=${encodeURIComponent(url.pathname)}`);
  }

  return {
    user: locals.session.profile,
    mfa: await idpFetch(locals, '/me/mfa/methods')
  };
};
