import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	sessionCode: string;
	clientId: string;
};

type Response = {
	/**
	 * The session code ROTATES on every refresh — the one we sent is dead the moment
	 * this returns. Callers must write this back to sessionStorage before the next
	 * call, or the flow will fail with "invalid or expired session".
	 */
	sessionCode: string;
	/** ISO-8601. The new idle deadline. */
	sessionExpiresAt: string;
	/** ISO-8601. Unchanged by a refresh — the ceiling is fixed at login. */
	sessionAbsoluteExpiresAt: string;
};

/**
 * Slides a live sign-in session's idle deadline forward because the user said they
 * are still there.
 *
 * Call this from an explicit user gesture only — never a background interval. An idle
 * timeout that renews itself while nobody is watching is not an idle timeout. The
 * server clamps the new deadline to an absolute ceiling and rejects the call once that
 * ceiling passes, so the session dies eventually no matter how often this is hit.
 */
export async function refreshSessionApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/session/refresh', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
