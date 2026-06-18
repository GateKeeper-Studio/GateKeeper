import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	passwordResetToken: string;
	passwordResetId: string;
	newPassword: string;
};

export async function resetPasswordApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/v1/auth/reset-password', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
