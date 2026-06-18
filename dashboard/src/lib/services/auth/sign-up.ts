import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	displayName: string;
	firstName: string;
	lastName: string;
	email: string;
	password: string;
};

export async function signUpApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/authentication/register', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
