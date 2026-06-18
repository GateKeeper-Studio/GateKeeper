import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Response = {
	access_token: string;
	token_type: string;
	expires_in: number;
	refresh_token: string;
};

/**
 * Platform-admin sign-in. Exchanges email + password for a short-lived HS256
 * bearer token (plus a long-lived refresh token) that the API's RBAC middleware
 * (WI-08) accepts on every admin route. The caller stores them via
 * `setAccessToken` / `setRefreshToken` so `api` replays and silently renews them.
 */
export async function adminLogin(email: string, password: string): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/admin/login', { email, password });
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

/**
 * Server-side sign-out: revokes the refresh token's whole rotation family so a
 * leaked copy can't be exchanged after logout. Idempotent and best-effort — the
 * caller clears local tokens regardless of the outcome.
 */
export async function adminLogout(refreshToken: string): Promise<void> {
	try {
		await api.post<void>('/admin/logout', { refresh_token: refreshToken });
	} catch {
		// Best-effort: a failed server revocation must not block local sign-out.
	}
}
