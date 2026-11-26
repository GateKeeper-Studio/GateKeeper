import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// The site root belongs to no page: the console lives under /console, the hosted
// login under /auth/[clientId].
export const load: PageLoad = ({ url }) => {
	redirect(307, `/console${url.search}`);
};
