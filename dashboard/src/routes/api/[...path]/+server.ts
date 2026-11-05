/**
 * Same-origin proxy for the admin API.
 *
 * The browser calls `/api/tenants/...`; this attaches the session's bearer token
 * and forwards to the IdP. The token itself never reaches the browser, which is
 * the whole point of the BFF — see `$lib/server/session`.
 *
 * Only the admin surface goes through here. The hosted end-user auth flow under
 * `/auth/[clientId]` keeps calling the IdP directly from the browser: those calls
 * carry no long-lived credential, and proxying them would hide the real client IP
 * from the IdP's risk engine at exactly the moment it matters (an interactive
 * login). See `$lib/services/base/api.ts`.
 */

import { error, type RequestHandler } from '@sveltejs/kit';
import { callIdp } from '$lib/server/idp';

const FORWARDED_METHODS = new Set(['GET', 'POST', 'PUT', 'DELETE', 'PATCH']);

const handler: RequestHandler = async ({ request, params, url, locals, getClientAddress }) => {
	if (!locals.sessionId) {
		// Not signed in. 401 rather than a redirect: the caller is `fetch`, and the
		// client wrapper turns this into an ApiError the UI can act on.
		error(401, 'not signed in');
	}

	if (!FORWARDED_METHODS.has(request.method)) {
		error(405, 'method not allowed');
	}

	// `params.path` is the remainder after /api/. Query string is carried across
	// verbatim so pagination and filters keep working.
	const target = `/${params.path ?? ''}${url.search}`;

	const contentType = request.headers.get('content-type');
	const body =
		request.method === 'GET' || request.method === 'DELETE' ? undefined : await request.text();

	const response = await callIdp(locals.sessionId, {
		method: request.method,
		path: target,
		body,
		contentType,
		clientAddress: safeClientAddress(getClientAddress)
	});

	return new Response(response.body || null, {
		status: response.status,
		headers: response.contentType ? { 'content-type': response.contentType } : undefined
	});
};

// `getClientAddress()` throws when the adapter cannot determine an address.
// The IP is an enrichment here, not a requirement, so degrade rather than 500.
function safeClientAddress(getClientAddress: () => string): string | null {
	try {
		return getClientAddress();
	} catch {
		return null;
	}
}

export const GET = handler;
export const POST = handler;
export const PUT = handler;
export const DELETE = handler;
export const PATCH = handler;
