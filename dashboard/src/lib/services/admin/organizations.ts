import { api } from '../base/api';
import { ApiError, type Result } from '../types';

// Mirrors the IdP's organization Response (camelCase projection). An admin is bound
// to exactly one organization via its token, so there is no list/create/delete
// surface here — only reading and updating that single org.
export type OrganizationView = {
	organizationId: string;
	name: string;
	description: string | null;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

export type OrganizationInput = {
	name: string;
	description: string | null;
	isActive: boolean;
};

// GET /admin/me/organization — the signed-in admin's own organization.
export async function getMyOrganization(
	fetchFn: typeof fetch = fetch
): Promise<Result<OrganizationView>> {
	try {
		const data = await api.get<OrganizationView>('/admin/me/organization', fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /admin/me/organization — update the admin's own organization.
export async function updateMyOrganization(
	input: OrganizationInput
): Promise<Result<OrganizationView>> {
	try {
		const data = await api.put<OrganizationView>('/admin/me/organization', {
			name: input.name,
			description: input.description,
			is_active: input.isActive
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
