import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	clientId: string;
	token: string;
	email: string;
	codeChallengeMethod: string;
	responseType: string;
	scope: string;
	state: string;
	codeChallenge: string;
	redirectUri: string;
	nonce: string;
};

type Response = {
	authorizationCode: string;
	state: string;
	codeChallenge: string;
	codeChallengeMethod: string;
	responseType: string;
	redirectUri: string;
};

export async function confirmEmailApi({ nonce, ...rest }: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/authentication/confirm-email', {
			...rest,
			...(nonce ? { nonce } : {})
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
