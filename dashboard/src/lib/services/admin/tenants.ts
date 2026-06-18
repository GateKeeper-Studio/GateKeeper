import { api } from '../base/api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's create-tenant Response (snake_case from the Rust API).
export type Tenant = {
	tenant_id: string;
	name: string;
	description: string | null;
	is_active: boolean;
	can_self_register: boolean;
	can_self_forgot_password: boolean;
	created_at: string;
};

export type CreateTenantInput = {
	subscriptionId: string;
	name: string;
	description?: string | null;
	passwordHashSecret: string;
	canSelfRegister: boolean;
	canSelfForgotPassword: boolean;
};

// Mirrors the IdP's list-tenants Response (camelCase — that projection renames).
export type TenantView = {
	tenantId: string;
	name: string;
	description: string | null;
	isActive: boolean;
	canSelfRegister: boolean;
	canSelfForgotPassword: boolean;
	createdAt: string;
	updatedAt: string | null;
};

// Mirrors the IdP's get-tenant-by-id Response (snake_case — no camelCase rename).
export type TenantDetail = {
	tenant_id: string;
	name: string;
	description: string | null;
	is_active: boolean;
	can_self_register: boolean;
	can_self_forgot_password: boolean;
	subscription_id: string;
	// True for the subscription's "master" tenant, which cannot be deleted directly.
	is_default: boolean;
	created_at: string;
	updated_at: string | null;
};

export type UpdateTenantInput = {
	name: string;
	description: string | null;
	isActive: boolean;
	canSelfRegister: boolean;
	canSelfForgotPassword: boolean;
};

type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

// GET /tenants/{tenantId} — full record for a single tenant. Powers the tenant
// overview page, which works even on a hard refresh (no reliance on the sidebar list).
export async function getTenant(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<TenantDetail>> {
	try {
		const data = await api.get<TenantDetail>(`/tenants/${tenantId}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants — lists every tenant the admin can manage. Pulls a large first
// page so the sidebar switcher has the full set without client-side paging.
export async function listTenants(
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<TenantView>>> {
	try {
		const data = await api.get<Paged<TenantView>>('/tenants?page=1&page_size=100', fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /tenants/{tenantId} — updates a tenant's editable fields. The password hash
// secret is fixed at creation and is not part of an update.
export async function updateTenant(
	tenantId: string,
	input: UpdateTenantInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}`, {
			name: input.name,
			description: input.description,
			is_active: input.isActive,
			can_self_register: input.canSelfRegister,
			can_self_forgot_password: input.canSelfForgotPassword
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// DELETE /tenants/{tenantId} — removes a tenant and everything under it (users,
// clients, …). The subscription's default ("master") tenant cannot be deleted.
export async function deleteTenant(tenantId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// POST /tenants — registers a new tenant. The password hash secret is what the
// IdP uses to hash every password belonging to users of this tenant.
export async function createTenant(
	input: CreateTenantInput,
	fetchFn: typeof fetch = fetch
): Promise<Result<Tenant>> {
	try {
		const data = await api.post<Tenant>(
			'/tenants',
			{
				subscription_id: input.subscriptionId,
				name: input.name,
				description: input.description ?? null,
				password_hash_secret: input.passwordHashSecret,
				can_self_register: input.canSelfRegister,
				can_self_forgot_password: input.canSelfForgotPassword
			},
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
