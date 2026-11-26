import { browser } from '$app/environment';
import { redirect } from '@sveltejs/kit';

import type { LayoutLoad } from './$types';
import { getClientAuthData } from '$lib/services/auth/get-client-auth-data';
import { baseLocale, cookieMaxAge, cookieName } from '$lib/paraglide/runtime';

// Pages that only make sense when the client accepts passwords. An external-login-only
// client has no password to create or reset, and the IdP refuses both endpoints — so
// send the user to the one screen that can actually sign them in.
const PASSWORD_ONLY_PAGES = ['/sign-up', '/forgot-password'];

/**
 * Loads the public auth data for the client once for every page in the auth
 * flow, from the identity provider on this same origin. Errors are returned (not
 * thrown) as a plain object so each page can render an `ErrorAlert` while still
 * showing the layout shell.
 */
export const load: LayoutLoad = async ({ params, fetch, url }) => {
	const [client, error] = await getClientAuthData(params.clientId, fetch);

	// Carry `url.search` across: it holds the RP's OAuth state, PKCE challenge and
	// redirect_uri. Dropping it would strand the user on a sign-in page that can't
	// complete the authorization request it was opened for.
	if (client?.externalLoginOnly && PASSWORD_ONLY_PAGES.some((p) => url.pathname.endsWith(p))) {
		redirect(302, `/auth/${params.clientId}/sign-in${url.search}`);
	}

	// First visit, no explicit language preference yet: adopt this client's own
	// default language instead of the global base locale, so the very next
	// navigation (and the switcher's active marker) reflect it. This only sets the
	// cookie going forward — it doesn't change the locale this page rendered in.
	if (
		browser &&
		client &&
		client.defaultLanguage !== baseLocale &&
		!document.cookie.split('; ').some((c) => c.startsWith(`${cookieName}=`))
	) {
		document.cookie = `${cookieName}=${client.defaultLanguage}; path=/; max-age=${cookieMaxAge}; samesite=lax`;
	}

	return {
		clientId: params.clientId,
		client,
		loadError: error ? { message: error.message } : null
	};
};
