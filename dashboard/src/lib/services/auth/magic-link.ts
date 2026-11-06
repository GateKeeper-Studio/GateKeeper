import { api } from '../base/api';
import { ApiError, type Result } from '../types';
import type { EMfaType, LoginResponse } from './login';

type RequestMagicLinkRequest = {
	email: string;
	clientId: string;
	redirectUri: string;
	responseType: string;
	scope: string;
	state: string;
	codeChallenge: string;
	codeChallengeMethod: string;
	nonce?: string;
};

type RequestMagicLinkResponse = {
	/**
	 * The code of the session waiting for the click. Kept in THIS browser: the emailed
	 * link only redeems together with it.
	 */
	sessionCode: string;
	/** ISO-8601. When the waiting session — and with it the link — dies. */
	sessionExpiresAt: string;
	sessionAbsoluteExpiresAt: string;
};

/**
 * "Email me a sign-in link". The IdP answers 202 the same way for every email — the
 * account lookup and the email happen after the response.
 */
export async function requestMagicLinkApi(
	request: RequestMagicLinkRequest
): Promise<Result<RequestMagicLinkResponse>> {
	try {
		const data = await api.post<RequestMagicLinkResponse>('/v1/auth/magic-link', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

type RedeemMagicLinkRequest = {
	clientId: string;
	/** From the link's fragment. */
	id: string;
	token: string;
	/** The waiting session's code, from this browser's storage. */
	sessionCode: string;
	mfaMethods?: EMfaType[];
	mfaDefaultMethod?: EMfaType;
};

/** The link was opened: answers like a password login (MFA routing, then authorize). */
export async function redeemMagicLinkApi(
	request: RedeemMagicLinkRequest
): Promise<Result<LoginResponse>> {
	try {
		const data = await api.post<LoginResponse>('/v1/auth/magic-link/redeem', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
