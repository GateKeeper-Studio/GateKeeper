import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { websiteDocsTarget } from '$lib/server/legacy-redirect';

export const load: PageServerLoad = ({ params, url }) => {
	redirect(308, websiteDocsTarget(params.path, url));
};
