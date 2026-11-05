// Example guarded route. If there is no session, send the user to sign in and
// remember to bring them back here afterwards (returnTo=/protected).
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals, url }) => {
  if (!locals.session) {
    redirect(302, `/auth/sign-in?returnTo=${encodeURIComponent(url.pathname)}`);
  }
  return { user: locals.session.profile };
};
