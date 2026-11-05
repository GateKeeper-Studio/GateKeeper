/**
 * Server-side client for the identity provider's admin API.
 *
 * This is the only place the admin bearer token is attached to a request. The
 * browser talks to `/api/*` on this origin; this module talks to the IdP.
 */

import { env } from '$env/dynamic/private';
import { tokensFor, updateTokens } from './session';

/** Server-only base URL. Deliberately not `PUBLIC_`: the browser never calls it. */
export function idpBaseUrl(): string {
	return env.IDP_BASE_URL ?? 'http://localhost:8080';
}

/**
 * Forwards the real client IP so the IdP sees the browser, not this server.
 *
 * This matters more than it looks. The IdP keys its rate limiter on the left-most
 * X-Forwarded-For hop and feeds the same address into audit records and the
 * adaptive risk engine. Without this header every admin request would arrive
 * wearing the dashboard's IP, so one admin tripping the credential rate limit
 * would lock out all of them, and audit trails would name the wrong host.
 */
function forwardedHeaders(clientAddress: string | null): Record<string, string> {
	return clientAddress ? { 'x-forwarded-for': clientAddress } : {};
}

export type IdpResponse = {
	status: number;
	body: string;
	contentType: string | null;
};

/**
 * Calls the IdP with the session's access token, transparently refreshing once on
 * a 401.
 *
 * Mirrors the retry logic the browser client used to run (a single retry, never
 * for the auth endpoints themselves), moved server-side. The single-flight guard
 * is carried over too: a dashboard page fan-out can 401 on several requests at
 * once, and the IdP rotates the refresh token on every use — concurrent refreshes
 * would spend the same token twice and trip its reuse detection, which revokes
 * the whole token family and signs the admin out.
 */
export async function callIdp(
	sessionId: string,
	init: {
		method: string;
		path: string;
		body?: string;
		contentType?: string | null;
		clientAddress: string | null;
	}
): Promise<IdpResponse> {
	const first = await send(sessionId, init);
	if (first.status !== 401) return first;

	const refreshed = await refreshTokens(sessionId, init.clientAddress);
	if (!refreshed) return first;

	return send(sessionId, init);
}

async function send(sessionId: string, init: Parameters<typeof callIdp>[1]): Promise<IdpResponse> {
	const tokens = tokensFor(sessionId);
	if (!tokens)
		return { status: 401, body: '{"error":"session expired"}', contentType: 'application/json' };

	const headers: Record<string, string> = {
		authorization: `Bearer ${tokens.accessToken}`,
		...forwardedHeaders(init.clientAddress)
	};
	if (init.contentType) headers['content-type'] = init.contentType;

	const response = await fetch(`${idpBaseUrl()}${init.path}`, {
		method: init.method,
		headers,
		body: init.body
	});

	return {
		status: response.status,
		body: await response.text(),
		contentType: response.headers.get('content-type')
	};
}

// One in-flight refresh per session.
const refreshInFlight = new Map<string, Promise<boolean>>();

async function refreshTokens(sessionId: string, clientAddress: string | null): Promise<boolean> {
	const existing = refreshInFlight.get(sessionId);
	if (existing) return existing;

	const attempt = (async () => {
		const tokens = tokensFor(sessionId);
		if (!tokens) return false;

		try {
			const response = await fetch(`${idpBaseUrl()}/admin/refresh`, {
				method: 'POST',
				headers: { 'content-type': 'application/json', ...forwardedHeaders(clientAddress) },
				body: JSON.stringify({ refresh_token: tokens.refreshToken })
			});
			if (!response.ok) return false;

			const data = (await response.json()) as { access_token: string; refresh_token: string };
			updateTokens(sessionId, {
				accessToken: data.access_token,
				refreshToken: data.refresh_token
			});
			return true;
		} catch {
			// A network failure is not an auth failure; the caller returns the
			// original 401 and the admin retries.
			return false;
		}
	})();

	refreshInFlight.set(sessionId, attempt);
	try {
		return await attempt;
	} finally {
		refreshInFlight.delete(sessionId);
	}
}

/** Unauthenticated call, for sign-in and sign-out. */
export async function callIdpAnonymous(init: {
	method: string;
	path: string;
	body: string;
	clientAddress: string | null;
}): Promise<IdpResponse> {
	const response = await fetch(`${idpBaseUrl()}${init.path}`, {
		method: init.method,
		headers: { 'content-type': 'application/json', ...forwardedHeaders(init.clientAddress) },
		body: init.body
	});

	return {
		status: response.status,
		body: await response.text(),
		contentType: response.headers.get('content-type')
	};
}
