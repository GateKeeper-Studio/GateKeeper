import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';
import { ApiError } from '../types';

/**
 * Base URL of the GateKeeper identity-provider (Go) server.
 * Configure via the `PUBLIC_API_BASE_URL` environment variable.
 */
const BASE_URL = env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';

type FetchFn = typeof fetch;

/**
 * The admin access token (a JWT carrying the caller's roles), attached as
 * `Authorization: Bearer` on every admin API call so the now-RBAC-protected
 * endpoints (WI-08) accept the request.
 *
 * Set by the sign-in flow via `setAccessToken`. In the browser we fall back to a
 * persisted copy so the token survives reloads. NOTE: storing the token where JS
 * can read it trades some XSS exposure for working cross-origin client calls; a
 * fully hardened setup proxies API calls through the SvelteKit server with an
 * httpOnly cookie + `handleFetch` injection instead.
 */
const TOKEN_KEY = 'gk_access';
const REFRESH_KEY = 'gk_refresh';
let accessToken: string | null = null;
let refreshToken: string | null = null;

export function setAccessToken(token: string | null): void {
	accessToken = token;
	if (browser) {
		if (token) localStorage.setItem(TOKEN_KEY, token);
		else localStorage.removeItem(TOKEN_KEY);
	}
}

export function getAccessToken(): string | null {
	if (accessToken) return accessToken;
	if (browser) accessToken = localStorage.getItem(TOKEN_KEY);
	return accessToken;
}

/**
 * The admin refresh token: an opaque, long-lived credential (also kept in
 * localStorage) that buys a new access token at `/admin/refresh` when the short
 * one expires — so the operator isn't bounced to /login mid-session. It is rotated
 * on every use; we always persist whatever the server most recently handed back.
 */
export function setRefreshToken(token: string | null): void {
	refreshToken = token;
	if (browser) {
		if (token) localStorage.setItem(REFRESH_KEY, token);
		else localStorage.removeItem(REFRESH_KEY);
	}
}

export function getRefreshToken(): string | null {
	if (refreshToken) return refreshToken;
	if (browser) refreshToken = localStorage.getItem(REFRESH_KEY);
	return refreshToken;
}

/** Drop both tokens (failed refresh / sign-out). The route guard then redirects. */
function clearSession(): void {
	setAccessToken(null);
	setRefreshToken(null);
}

// Single-flight refresh: many requests can 401 at once (e.g. a dashboard fan-out),
// but we must rotate the refresh token exactly once. Concurrent callers share the
// one in-flight attempt.
let refreshInFlight: Promise<boolean> | null = null;

async function refreshAccessToken(): Promise<boolean> {
	const stored = getRefreshToken();
	if (!stored) return false;

	if (!refreshInFlight) {
		refreshInFlight = (async () => {
			try {
				const res = await fetch(`${BASE_URL}/admin/refresh`, {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ refresh_token: stored })
				});
				if (!res.ok) return false;
				const data = (await res.json()) as { access_token: string; refresh_token: string };
				setAccessToken(data.access_token);
				setRefreshToken(data.refresh_token);
				return true;
			} catch {
				return false;
			}
		})();
		void refreshInFlight.finally(() => {
			refreshInFlight = null;
		});
	}

	return refreshInFlight;
}

// The auth endpoints authenticate by themselves (credentials / refresh token), so a
// 401 from them is terminal — never trigger the refresh-and-retry dance for them.
function isAuthEndpoint(path: string): boolean {
	return (
		path.startsWith('/admin/login') ||
		path.startsWith('/admin/refresh') ||
		path.startsWith('/admin/logout')
	);
}

async function request<T>(
	method: string,
	path: string,
	body: unknown,
	fetchFn: FetchFn,
	isRetry = false
): Promise<T> {
	const headers: Record<string, string> = {};
	if (body !== undefined) headers['Content-Type'] = 'application/json';
	const token = getAccessToken();
	if (token) headers['Authorization'] = `Bearer ${token}`;

	const res = await fetchFn(`${BASE_URL}${path}`, {
		method,
		headers: Object.keys(headers).length > 0 ? headers : undefined,
		body: body !== undefined ? JSON.stringify(body) : undefined
	});

	if (!res.ok) {
		// Access token likely expired: rotate it via the refresh token and retry once.
		if (res.status === 401 && !isRetry && !isAuthEndpoint(path) && getRefreshToken()) {
			if (await refreshAccessToken()) {
				return request<T>(method, path, body, fetchFn, true);
			}
			clearSession(); // refresh failed — the route guard will bounce to /login
		}

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
 * Thin JSON fetch wrapper around the GateKeeper API.
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
