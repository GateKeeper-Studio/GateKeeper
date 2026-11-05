import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Account now lives in the global Settings dialog. Redirect legacy links, bookmarks and
// command-palette entries to `/?settings=account`, which pops the dialog open.
export const load: PageLoad = ({ url }) => {
	const params = new URLSearchParams(url.searchParams);
	params.set('settings', 'account');
	redirect(307, `/?${params}`);
};
