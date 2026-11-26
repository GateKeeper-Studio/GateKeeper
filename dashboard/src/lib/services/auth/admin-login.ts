import { ApiError, type Result } from '../types';
import type { AdminIdentity } from '$lib/types/admin';

type LoginResponse = { admin: AdminIdentity };

/**
 * Platform-admin sign-in.
 *
 * Posts to `/session` on this origin, which is the identity provider itself (it
 * serves this app). The server verifies the password, opens a console session and
 * returns only the admin's display identity plus an httpOnly cookie. No token ever
 * reaches the browser: admin API calls go to `/api/*`, where the server turns the
 * cookie into a short-lived bearer token.
 *
 * Previously this called `/admin/login` from the browser and the caller put both
 * tokens into localStorage, where any injected script could read them — and the
 * refresh token in particular turned a single XSS into indefinite account access.
 */
// Error values the server uses for the second step of sign-in.
export const MFA_REQUIRED = 'mfa_required';
export const INVALID_MFA_CODE = 'invalid_mfa_code';

/**
 * An admin with two-step verification gets `401 {error: "mfa_required"}` for a right
 * password without a code (never for a wrong password), and signs in by posting the
 * same email and password again with `mfaCode`: a code from the authenticator app or
 * a recovery code.
 */
export async function adminLogin(
	email: string,
	password: string,
	mfaCode?: string
): Promise<Result<LoginResponse>> {
	try {
		const res = await fetch('/session', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ email, password, mfaCode: mfaCode || undefined }),
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
 * Sign-out. The server deletes the session (so a leaked copy of the cookie is dead,
 * not merely forgotten) and clears the cookie. Best-effort by design.
 */
export async function adminLogout(): Promise<void> {
	try {
		await fetch('/session', { method: 'DELETE', credentials: 'same-origin' });
	} catch {
		// A failed revocation must not block sign-out.
	}
}

/**
 * Who is signed in, or `null` when nobody is (or the session expired).
 *
 * The console's route guard (`routes/console/+layout.ts`) calls this on every
 * navigation. It is a UX guard only: every piece of data the console shows comes
 * from `/api/*`, which the server refuses without a live session anyway.
 */
export async function getAdminSession(
	fetchFn: typeof fetch = fetch
): Promise<AdminIdentity | null> {
	const res = await fetchFn('/session', { credentials: 'same-origin' });
	if (res.status === 401) return null;
	if (!res.ok) throw new ApiError(res.status, {});
	const data = (await res.json()) as { admin: AdminIdentity };
	return data.admin;
}
