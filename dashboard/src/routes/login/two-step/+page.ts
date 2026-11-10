import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getAdminSession } from '$lib/services/auth/admin-login';

// Only for a signed-in admin whose organization requires two-step verification and
// who has not set it up yet; everyone else belongs elsewhere.
export const load: PageLoad = async ({ fetch }) => {
	const admin = await getAdminSession(fetch);
	if (!admin) redirect(302, '/login');
	if (!admin.mfaSetupRequired) redirect(302, '/console');
	return { admin };
};
