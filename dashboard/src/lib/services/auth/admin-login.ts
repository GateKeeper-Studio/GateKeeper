import { ApiError, type Result } from '../types';
import type { AdminIdentity } from '$lib/server/session';

type LoginResponse = { admin: AdminIdentity };

/**
 * Platform-admin sign-in.
 *
 * Posts to this app's own `/session` endpoint, NOT to the IdP. The server
 * exchanges the credentials for the IdP token pair, keeps both in the server-side
 * session, and returns only the admin's display identity plus an httpOnly cookie.
 *
 * Previously this called `/admin/login` from the browser and the caller put both
 * tokens into localStorage, where any injected script could read them — and the
 * refresh token in particular turned a single XSS into indefinite account access.
 */
export async function adminLogin(email: string, password: string): Promise<Result<LoginResponse>> {
	try {
		const res = await fetch('/session', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ email, password }),
			credentials: 'same-origin'
		});

		if (!res.ok) {
			let data: { message?: string; error?: string } = {};
			try {
				data = await res.json();
			} catch {
				// Non-JSON error body — keep the empty object.
			}
			return [null, new ApiError(res.status, data)];
		}

		return [(await res.json()) as LoginResponse, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

/**
 * Sign-out. The server revokes the refresh token's whole rotation family at the
 * IdP (so a leaked copy is dead, not merely forgotten) and clears the cookie.
 * Best-effort by design: the local session goes regardless of the IdP's answer.
 */
export async function adminLogout(): Promise<void> {
	try {
		await fetch('/session', { method: 'DELETE', credentials: 'same-origin' });
	} catch {
		// A failed revocation must not block sign-out.
	}
}
