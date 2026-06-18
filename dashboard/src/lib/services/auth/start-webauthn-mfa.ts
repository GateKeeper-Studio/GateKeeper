import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	clientId: string;
	sessionCode: string;
};

type Response = {
	mfaId: string;
	options: unknown;
};

export async function startWebAuthnMfaApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/verify-mfa/webauthn/start', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

