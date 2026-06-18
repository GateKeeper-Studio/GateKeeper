import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	email: string;
	sessionCode: string;
	clientId: string;
	redirectUri: string;
	codeChallengeMethod: string;
	responseType: string;
	scope: string;
	state: string;
	codeChallenge: string;
	nonce?: string;
	mfaId?: string;
};

type Response = {
	authorizationCode: string;
	state: string;
	codeChallenge: string;
	codeChallengeMethod: string;
	responseType: string;
	redirectUri: string;
};

export async function authorizeApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/authorize', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
