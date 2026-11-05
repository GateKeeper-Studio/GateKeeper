import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's ExternalProviderView (camelCase). The client secret is never
// returned — it is write-only over the API.
export type IdentityProvider = {
	id: string;
	oauthClientId: string;
	provider: string; // 'google' | 'github'
	providerClientId: string;
	redirectUri: string;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

export type IdentityProviderInput = {
	provider: string; // 'google' | 'github'
	clientId: string;
	clientSecret?: string; // required on create; optional on update (kept if omitted)
	redirectUri: string;
	isActive: boolean;
};

export async function listIdentityProviders(
	tenantId: string,
	clientId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<IdentityProvider[]>> {
	try {
		const data = await api.get<IdentityProvider[]>(
			`/tenants/${tenantId}/oauth-clients/${clientId}/identity-providers`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function createIdentityProvider(
	tenantId: string,
	clientId: string,
	input: IdentityProviderInput
): Promise<VoidResult> {
	try {
		await api.post(`/tenants/${tenantId}/oauth-clients/${clientId}/identity-providers`, input);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function updateIdentityProvider(
	tenantId: string,
	clientId: string,
	id: string,
	input: IdentityProviderInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/oauth-clients/${clientId}/identity-providers/${id}`, input);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteIdentityProvider(
	tenantId: string,
	clientId: string,
	id: string
): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/oauth-clients/${clientId}/identity-providers/${id}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
