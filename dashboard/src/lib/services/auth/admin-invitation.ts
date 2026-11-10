import { ApiError, type Result } from '../types';

// The public half of an admin invitation. The link's token (kept in the URL
// fragment, which never reaches a server log) is the credential; both calls sit in
// the server's strict sign-in rate limit.

export type InvitationDetails = {
	email: string;
	organizationName: string;
	expiresAt: string;
};

async function post<T>(path: string, body: unknown): Promise<Result<T>> {
	try {
		const res = await fetch(path, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify(body),
			credentials: 'same-origin'
		});
		let data: unknown = null;
		try {
			data = await res.json();
		} catch {
			// Non-JSON body.
		}
		if (!res.ok) return [null, new ApiError(res.status, (data ?? {}) as ApiError['data'])];
		return [data as T, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export const lookupInvitation = (token: string) =>
	post<InvitationDetails>('/admin/invitations/lookup', { token });

export const acceptInvitation = (token: string, name: string, password: string) =>
	post<{ email: string }>('/admin/invitations/accept', { token, name, password });
