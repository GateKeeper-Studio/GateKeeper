import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	providerName: string;
	oauthProviderId: string;
	clientCodeChallenge: string;
	clientCodeChallengeMethod: string;
	clientRedirectUri: string;
	clientState: string;
	clientResponseType: string;
	clientScope: string;
	clientNonce?: string;
};

type Response = {
	url: string;
};

export async function oauthProviderLoginApi({
	providerName,
	...body
}: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>(`/v1/auth/oauth-provider/${providerName}/login`, body);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
