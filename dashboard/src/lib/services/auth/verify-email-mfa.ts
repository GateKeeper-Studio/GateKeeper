import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	clientId: string;
	sessionCode: string;
	code: string;
};

type Response = {
	sessionCode: string;
};

export async function verifyEmailMfaApi(request: Request): Promise<Result<Response>> {
	try {
		const data = await api.post<Response>('/v1/auth/verify-mfa/email', request);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
