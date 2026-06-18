import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	email: string;
};

export async function forgotPasswordApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/v1/auth/forgot-password', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
