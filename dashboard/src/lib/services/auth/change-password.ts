import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	userId: string;
	/** The sign-in session the temporary password opened (past its second factor). */
	sessionCode: string;
	changePasswordCode: string;
	newPassword: string;
};

export async function changePasswordApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/v1/auth/change-password', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
