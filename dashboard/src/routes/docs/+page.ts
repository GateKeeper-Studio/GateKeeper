import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { websiteDocsTarget } from '$lib/legacy-redirect';

export const load: PageLoad = ({ url }) => {
	redirect(308, websiteDocsTarget(undefined, url));
};
