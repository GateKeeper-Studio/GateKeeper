import { api } from '../base/api';
import { ApiError, type Result } from '../types';

type Request = {
	clientId: string;
	sessionCode: string;
};

export async function sendEmailMfaApi(request: Request): Promise<Result<void>> {
	try {
		await api.post<void>('/v1/auth/verify-mfa/email/send', request);
		return [undefined, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

