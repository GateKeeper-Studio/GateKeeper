import { api } from '../base/api';
import { ApiError, type Result } from '../types';

export enum EMfaType {
	MfaApp = 'totp',
	MfaEmail = 'email',
	MfaWebauthn = 'webauthn'
}

type Request = {
	email: string;
	/**
	 * Omit for passwordless login: an account with no password and a confirmed
	 * passkey then gets a WebAuthn challenge (`mfaType: 'webauthn'`) instead of a
	 * session, and every other account the same 401 a wrong password gets.
	 */
	password?: string;
	clientId: string;
	redirectUri: string;
	codeChallengeMethod: string;
	responseType: string;
	scope: string;
	state: string;
	codeChallenge: string;
	nonce?: string;
	mfaMethods?: EMfaType[];
	mfaDefaultMethod?: EMfaType;
};

type Response = {
	mfaType?: EMfaType | null;
	mfaId?: string;
	message: string;
	sessionCode: string;
	/** Absent on a passwordless login: the account id is not revealed before the passkey verifies. */
	userId?: string;
	changePasswordCode?: string | null;
	webAuthnOptions?: unknown;
	availableMfaMethods?: EMfaType[];
	mfaEnrollmentRequired?: boolean;
	enrollableMethods?: EMfaType[];
	/** ISO-8601. The idle deadline; `refreshSessionApi` pushes it forward. */
	sessionExpiresAt: string;
	/** ISO-8601. The ceiling a refresh can never cross — when to stop offering one. */
	sessionAbsoluteExpiresAt: string;
};

export async function loginApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/login', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
