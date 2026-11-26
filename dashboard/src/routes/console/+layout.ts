import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { getAdminSession } from '$lib/services/auth/admin-login';

/**
 * The console guard.
 *
 * The console's pages are public static files, so this guard is about UX, not
 * secrecy: it sends a signed-out visitor to /login instead of showing a shell full
 * of 401s. The protection itself is server-side — every piece of data comes from
 * `/api/*`, which the identity provider refuses without a live session cookie.
 */
export const load: LayoutLoad = async ({ fetch, url }) => {
	const admin = await getAdminSession(fetch);
	if (!admin) {
		// Carry the attempted path so sign-in can return the admin to it. Only the
		// path+query is kept — never an absolute URL — so this cannot be turned into
		// an open redirect.
		const target = `${url.pathname}${url.search}`;
		const next = target === '/console' ? '' : `?next=${encodeURIComponent(target)}`;
		redirect(302, `/login${next}`);
	}

	// The organization requires two-step verification and this admin has none yet:
	// the server refuses everything but enrollment, so go straight there.
	if (admin.mfaSetupRequired) redirect(302, '/login/two-step');

	return { admin };
};
