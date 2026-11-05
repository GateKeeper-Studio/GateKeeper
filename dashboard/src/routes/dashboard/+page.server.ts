import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { legacyDashboardTarget } from '$lib/server/legacy-redirect';

export const load: PageServerLoad = ({ url }) => {
	redirect(308, legacyDashboardTarget(undefined, url));
};
