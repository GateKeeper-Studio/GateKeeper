import { api } from '../base/api';
import { ApiError, type Result } from '../types';

export type ClientAuthData = {
	id: string;
	name: string;
	canSelfSignUp: boolean;
	canSelfForgotPass: boolean;
	tosUri: string | null;
	policyUri: string | null;
	oauthProviders: {
		id: string;
		name: string;
	}[];
};

/**
 * Fetches the public auth configuration for a client (name, self-service flags
 * and configured OAuth providers). Pass `event.fetch` when called from a
 * SvelteKit `load` function.
 */
export async function getClientAuthData(
	clientId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<ClientAuthData>> {
	try {
		const data = await api.get<ClientAuthData>(`/v1/auth/client/${clientId}/auth-data`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
