import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { legacyDashboardTarget } from '$lib/legacy-redirect';

export const load: PageLoad = ({ params, url }) => {
	redirect(308, legacyDashboardTarget(params.path, url));
};
