/**
 * The dashboard's sign-in / sign-out endpoint.
 *
 * POST exchanges credentials for an IdP token pair and keeps BOTH server-side,
 * handing the browser only an httpOnly session cookie. The response body carries
 * the admin's display identity and never a token — that is the difference from
 * the old flow, where `/admin/login`'s response went straight into localStorage.
 *
 * DELETE revokes the refresh-token family at the IdP before clearing the cookie,
 * so a copy that leaked earlier is dead rather than merely forgotten.
 */

import { json, type RequestHandler } from '@sveltejs/kit';
import { callIdpAnonymous } from '$lib/server/idp';
import {
	createSession,
	destroySession,
	identityFromAccessToken,
	tokensFor
} from '$lib/server/session';

function safeClientAddress(getClientAddress: () => string): string | null {
	try {
		return getClientAddress();
	} catch {
		return null;
	}
}

export const POST: RequestHandler = async ({ request, cookies, getClientAddress }) => {
	const { email, password } = (await request.json()) as { email?: string; password?: string };
	if (!email || !password) {
		return json({ error: 'email and password are required' }, { status: 400 });
	}

	const clientAddress = safeClientAddress(getClientAddress);
	const response = await callIdpAnonymous({
		method: 'POST',
		path: '/admin/login',
		body: JSON.stringify({ email, password }),
		clientAddress
	});

	if (response.status !== 200) {
		// Pass the IdP's status through untouched. It answers a wrong password and an
		// unknown address identically (401, generic message) so as not to reveal which
		// admin accounts exist, and 429 when the credential rate limit trips — both
		// need to reach the sign-in form as-is.
		return new Response(response.body || null, {
			status: response.status,
			headers: response.contentType ? { 'content-type': response.contentType } : undefined
		});
	}

	const tokens = JSON.parse(response.body) as { access_token: string; refresh_token: string };
	const admin = identityFromAccessToken(tokens.access_token);
	if (!admin) {
		return json({ error: 'identity provider returned an unreadable token' }, { status: 502 });
	}

	createSession(
		cookies,
		{ accessToken: tokens.access_token, refreshToken: tokens.refresh_token },
		admin
	);

	return json({ admin });
};

export const DELETE: RequestHandler = async ({ cookies, locals, getClientAddress }) => {
	const tokens = locals.sessionId ? tokensFor(locals.sessionId) : null;

	if (tokens) {
		// Best-effort: a failed server-side revocation must not block sign-out.
		try {
			await callIdpAnonymous({
				method: 'POST',
				path: '/admin/logout',
				body: JSON.stringify({ refresh_token: tokens.refreshToken }),
				clientAddress: safeClientAddress(getClientAddress)
			});
		} catch {
			// Ignored deliberately; the cookie and server record go regardless.
		}
	}

	destroySession(cookies, locals.sessionId);
	return json({ ok: true });
};
