import { redirect } from '@sveltejs/kit';

import type { LayoutServerLoad } from './$types';
import { getClientAuthData } from '$lib/services/auth/get-client-auth-data';
import { baseLocale, cookieName } from '$lib/paraglide/runtime';

// Pages that only make sense when the client accepts passwords. An external-login-only
// client has no password to create or reset, and the IdP refuses both endpoints — so
// send the user to the one screen that can actually sign them in.
const PASSWORD_ONLY_PAGES = ['/sign-up', '/forgot-password'];

/**
 * Loads the public auth data for the client once for every page in the auth
 * flow. Runs only on the server: the browser never calls the API for this data —
 * it receives the result serialized in the page payload. Errors are returned
 * (not thrown) as a plain, serializable object so each page can render an
 * `ErrorAlert` while still showing the layout shell.
 */
export const load: LayoutServerLoad = async ({ params, fetch, cookies, url }) => {
	const [client, error] = await getClientAuthData(params.clientId, fetch);

	// Carry `url.search` across: it holds the RP's OAuth state, PKCE challenge and
	// redirect_uri. Dropping it would strand the user on a sign-in page that can't
	// complete the authorization request it was opened for.
	if (client?.externalLoginOnly && PASSWORD_ONLY_PAGES.some((p) => url.pathname.endsWith(p))) {
		redirect(302, `/auth/${params.clientId}/sign-in${url.search}`);
	}

	// First visit, no explicit language preference yet: adopt this client's own
	// default language instead of the global base locale, so the very next
	// navigation (and the switcher's active marker) reflect it. This only sets
	// the cookie going forward — it doesn't change which locale THIS response
	// was already resolved/rendered in.
	if (client && !cookies.get(cookieName) && client.defaultLanguage !== baseLocale) {
		cookies.set(cookieName, client.defaultLanguage, { path: '/' });
	}

	return {
		clientId: params.clientId,
		client,
		loadError: error ? { message: error.message } : null
	};
};
