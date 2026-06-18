import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	email: string;
};

export async function resendConfirmEmailApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/v1/auth/confirm-email/resend', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
