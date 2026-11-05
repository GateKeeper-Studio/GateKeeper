import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

/**
 * The admin guard.
 *
 * This replaces a client-side check (`if (browser && !getAccessToken())`) that
 * could only run after hydration — so the dashboard shell was server-rendered for
 * unauthenticated requests and the redirect happened in the browser, and any
 * `+page.server.ts` added under this group would have run with no auth check at
 * all. Running it here means nothing renders and no load function executes until
 * the session is proven.
 *
 * The session comes from `locals`, populated in `hooks.server.ts` from the
 * httpOnly cookie, so it cannot be forged by the client.
 */
export const load: LayoutServerLoad = async ({ locals, url }) => {
	if (!locals.admin) {
		// Carry the attempted path so sign-in can return the admin to it. Only the
		// path+query is kept — never an absolute URL — so this cannot be turned into
		// an open redirect.
		const target = `${url.pathname}${url.search}`;
		const next = target === '/' ? '' : `?next=${encodeURIComponent(target)}`;
		redirect(302, `/login${next}`);
	}

	return { admin: locals.admin };
};
