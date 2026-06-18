import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// The dashboard is the app's home; its layout guard handles the sign-in redirect.
export const load: PageLoad = () => {
	redirect(302, '/dashboard');
};
