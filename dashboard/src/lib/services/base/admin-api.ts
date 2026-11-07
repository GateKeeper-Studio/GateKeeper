import { ApiError } from '../types';

/**
 * Client for the admin API, through the identity provider's console bridge.
 *
 * Requests go to `/api/*` on this origin (the identity provider serves this app).
 * The server resolves the httpOnly session cookie, mints a short-lived admin bearer
 * token and hands the request to its admin API (server: `console_session.rs`).
 * Nothing here touches a token, because the browser never has one.
 *
 * The exported shape matches `base/api.ts` deliberately, so the admin services
 * differ only in which client they import.
 */

const BASE_PATH = '/api';

type FetchFn = typeof fetch;

async function request<T>(
	method: string,
	path: string,
	body: unknown,
	fetchFn: FetchFn
): Promise<T> {
	const headers: Record<string, string> = {};
	if (body !== undefined) headers['Content-Type'] = 'application/json';

	const res = await fetchFn(`${BASE_PATH}${path}`, {
		method,
		headers: Object.keys(headers).length > 0 ? headers : undefined,
		body: body !== undefined ? JSON.stringify(body) : undefined,
		// Same-origin, so the session cookie rides along by default; stated
		// explicitly because this is the only thing authenticating the call.
		credentials: 'same-origin'
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

export const adminApi = {
	get: <T>(path: string, fetchFn: FetchFn = fetch) => request<T>('GET', path, undefined, fetchFn),
	post: <T>(path: string, body?: unknown, fetchFn: FetchFn = fetch) =>
		request<T>('POST', path, body, fetchFn),
	put: <T>(path: string, body?: unknown, fetchFn: FetchFn = fetch) =>
		request<T>('PUT', path, body, fetchFn),
	delete: <T>(path: string, fetchFn: FetchFn = fetch) =>
		request<T>('DELETE', path, undefined, fetchFn)
};
