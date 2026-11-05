import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Organization now lives in the global Settings dialog. Redirect legacy links to
// `/?settings=organization`, which pops the dialog open on that section.
export const load: PageLoad = ({ url }) => {
	const params = new URLSearchParams(url.searchParams);
	params.set('settings', 'organization');
	redirect(307, `/?${params}`);
};
