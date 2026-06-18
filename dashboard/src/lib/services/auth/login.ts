import { api } from '../base/api';
import { ApiError, type Result } from '../types';

export enum EMfaType {
	MfaApp = 'totp',
	MfaEmail = 'email',
	MfaWebauthn = 'webauthn'
}

type Request = {
	email: string;
	password: string;
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
	userId: string;
	changePasswordCode?: string | null;
	webAuthnOptions?: unknown;
	availableMfaMethods?: EMfaType[];
	mfaEnrollmentRequired?: boolean;
	enrollableMethods?: EMfaType[];
};

export async function loginApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/login', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
