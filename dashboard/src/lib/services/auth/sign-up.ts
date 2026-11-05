import { api } from '../base/api';
import { ApiError, type VoidResult } from '../types';

type Request = {
	clientId: string;
	displayName: string;
	firstName: string;
	lastName: string;
	email: string;
	password: string;
	// Raw answers to the tenant's custom sign-up fields, keyed by field key. The
	// wire is camelCase here (register::Command has a camelCase rename, unlike
	// most other write commands in this API) so this is sent through as-is.
	customFieldValues?: Record<string, unknown> | null;
};

export async function signUpApi(request: Request): Promise<VoidResult> {
	try {
		await api.post('/authentication/register', request);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
