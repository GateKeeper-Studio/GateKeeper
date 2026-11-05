import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { websiteDocsTarget } from '$lib/server/legacy-redirect';

export const load: PageServerLoad = ({ url }) => {
	redirect(308, websiteDocsTarget(undefined, url));
};
