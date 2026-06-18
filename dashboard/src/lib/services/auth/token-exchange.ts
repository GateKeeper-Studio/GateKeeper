import { env } from '$env/dynamic/public';
import { ApiError, type Result } from '../types';

const BASE_URL = env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';

export type TokenResponse = {
	access_token: string;
	id_token?: string;
	refresh_token: string;
	token_type: string;
	expires_in: number;
	scope: string;
};

// POST /oauth/token (grant_type=authorization_code) — exchanges the authorization
// code obtained from the login → authorize flow for an access token. This is the
// final step that yields the admin token the dashboard attaches via setAccessToken.
//
// The OAuth token endpoint is form-encoded (RFC 6749 §4.1.3).
export async function exchangeCodeForToken(
	params: {
		tenantId: string;
		clientId: string;
		code: string;
		redirectUri: string;
		codeVerifier: string;
		clientSecret?: string;
	},
	fetchFn: typeof fetch = fetch
): Promise<Result<TokenResponse>> {
	const body = new URLSearchParams();
	body.set('grant_type', 'authorization_code');
	body.set('tenant_id', params.tenantId);
	body.set('client_id', params.clientId);
	body.set('code', params.code);
	body.set('redirect_uri', params.redirectUri);
	body.set('code_verifier', params.codeVerifier);
	if (params.clientSecret) body.set('client_secret', params.clientSecret);

	try {
		const res = await fetchFn(`${BASE_URL}/oauth/token`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
			body: body.toString()
		});
		if (!res.ok) {
			let data: { error?: string; message?: string } = {};
			try {
				data = await res.json();
			} catch {
				// keep empty
			}
			return [null, new ApiError(res.status, { error: data.error ?? 'token_error', message: data.message })];
		}
		return [(await res.json()) as TokenResponse, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /oauth/token (grant_type=refresh_token) — rotates the access token.
export async function refreshAccessToken(
	params: { tenantId: string; clientId: string; refreshToken: string; clientSecret?: string },
	fetchFn: typeof fetch = fetch
): Promise<Result<TokenResponse>> {
	const body = new URLSearchParams();
	body.set('grant_type', 'refresh_token');
	body.set('tenant_id', params.tenantId);
	body.set('client_id', params.clientId);
	body.set('refresh_token', params.refreshToken);
	if (params.clientSecret) body.set('client_secret', params.clientSecret);

	try {
		const res = await fetchFn(`${BASE_URL}/oauth/token`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
			body: body.toString()
		});
		if (!res.ok) {
			return [null, new ApiError(res.status, { error: 'refresh_failed' })];
		}
		return [(await res.json()) as TokenResponse, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
