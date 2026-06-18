import { browser } from '$app/environment';
import { redirect } from '@sveltejs/kit';
import { getAccessToken } from '$lib/services/base/api';
import type { LayoutLoad } from './$types';

// Client-side admin guard: the admin token lives in localStorage (see base/api.ts),
// so we can only check it in the browser. With no token, bounce to the sign-in page
// before any dashboard data loads. SSR renders through (browser === false); the
// check then runs on hydration. A 401 from the API also lands the user here.
export const load: LayoutLoad = () => {
	if (browser && !getAccessToken()) {
		redirect(302, '/login');
	}
};
