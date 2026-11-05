import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { legacyDashboardTarget } from '$lib/server/legacy-redirect';

export const load: PageServerLoad = ({ params, url }) => {
	redirect(308, legacyDashboardTarget(params.path, url));
};
