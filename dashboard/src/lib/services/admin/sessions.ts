import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

/**
 * One signed-in session: an app the user is signed in to (an active refresh-token
 * family). Deliberately minimal — the IdP stores no IP or user agent for it.
 */
export type SessionView = {
	/** The session id; pass it to `revokeUserSession`. */
	id: string;
	/** The app's public client_id. */
	client_id: string;
	client_name: string | null;
	scopes: string[];
	signed_in_at: string;
	/** The last token refresh — only as precise as the access-token lifetime. */
	last_active_at: string;
	expires_at: string;
};

const base = (tenantId: string, userId: string) => `/tenants/${tenantId}/users/${userId}/sessions`;

// GET /tenants/{tenantId}/users/{userId}/sessions
export async function listUserSessions(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<SessionView[]>> {
	try {
		const data = await api.get<SessionView[]>(base(tenantId, userId), fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/users/{userId}/sessions/{sessionId}
export async function revokeUserSession(
	tenantId: string,
	userId: string,
	sessionId: string
): Promise<VoidResult> {
	try {
		await api.delete(`${base(tenantId, userId)}/${sessionId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/users/{userId}/sessions — sign out everywhere.
export async function revokeAllUserSessions(tenantId: string, userId: string): Promise<VoidResult> {
	try {
		await api.delete(base(tenantId, userId));
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
