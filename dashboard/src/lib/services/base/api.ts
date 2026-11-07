import { ApiError } from '../types';

/**
 * Client for the identity provider's PUBLIC endpoints.
 *
 * Used only by the hosted end-user auth flow under `/auth/[clientId]` —
 * sign-in, sign-up, MFA verification, password reset, email confirmation. Those
 * endpoints authenticate by their own means (client id + PKCE + short-lived,
 * single-use session codes), so there is no bearer token to attach and nothing
 * long-lived to steal.
 *
 * The identity provider serves this app, so these are same-origin relative calls:
 * the browser talks to the IdP directly, and the risk engine and the per-IP rate
 * limits see the user's real address.
 *
 * The ADMIN surface is the opposite case — a long-lived credential worth
 * protecting — and goes through `base/admin-api.ts` and the server-side session
 * instead. Nothing in this file touches a token any more.
 */
// Same origin: the identity provider serves this app.
const BASE_URL = '';

type FetchFn = typeof fetch;

async function request<T>(
	method: string,
	path: string,
	body: unknown,
	fetchFn: FetchFn
): Promise<T> {
	const headers: Record<string, string> = {};
	if (body !== undefined) headers['Content-Type'] = 'application/json';

	const res = await fetchFn(`${BASE_URL}${path}`, {
		method,
		headers: Object.keys(headers).length > 0 ? headers : undefined,
		body: body !== undefined ? JSON.stringify(body) : undefined
	});

	if (!res.ok) {
		let data: { message?: string; title?: string } = {};
		try {
			data = await res.json();
		} catch {
			// Non-JSON error body — keep the empty object.
		}
		throw new ApiError(res.status, data);
	}

	if (res.status === 204 || res.headers.get('content-length') === '0') {
		return undefined as T;
	}

	return (await res.json()) as T;
}

/**
 * Thin JSON fetch wrapper around the GateKeeper public API.
 *
 * `fetchFn` defaults to the global `fetch`; pass SvelteKit's `event.fetch`
 * from `load` functions so SSR requests are tracked and deduplicated.
 */
export const api = {
	get: <T>(path: string, fetchFn: FetchFn = fetch) => request<T>('GET', path, undefined, fetchFn),
	post: <T>(path: string, body?: unknown, fetchFn: FetchFn = fetch) =>
		request<T>('POST', path, body, fetchFn),
	put: <T>(path: string, body?: unknown, fetchFn: FetchFn = fetch) =>
		request<T>('PUT', path, body, fetchFn),
	delete: <T>(path: string, fetchFn: FetchFn = fetch) =>
		request<T>('DELETE', path, undefined, fetchFn)
};
