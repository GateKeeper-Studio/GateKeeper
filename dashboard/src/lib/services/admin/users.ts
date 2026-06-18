import { api } from '../base/api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's tenant-user list/get projection (snake_case — that projection
// has no camelCase rename). `display_name` is optional; the rest are always present.
export type TenantUserView = {
	user_id: string;
	display_name: string | null;
	first_name: string;
	last_name: string;
	email: string;
	created_at: string;
	updated_at: string | null;
};

export type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

export type ListUsersParams = { page?: number; pageSize?: number };

export async function listTenantUsers(
	tenantId: string,
	params: ListUsersParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<TenantUserView>>> {
	const page = params.page ?? 1;
	const pageSize = params.pageSize ?? 100;
	try {
		const data = await api.get<Paged<TenantUserView>>(
			`/tenants/${tenantId}/users?page=${page}&page_size=${pageSize}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function getTenantUser(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<TenantUserView>> {
	try {
		const data = await api.get<TenantUserView>(
			`/tenants/${tenantId}/users/${userId}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// What the create endpoint accepts (camelCase here, mapped to the IdP's snake_case
// Command below). `temporaryPassword` is set once and the user must change it on
// first sign-in; `isEmailVerified` skips the confirmation email when true.
export type CreateTenantUserInput = {
	email: string;
	displayName: string | null;
	firstName: string;
	lastName: string;
	isEmailVerified: boolean;
	temporaryPassword: string;
};

// The editable details of an existing user. The IdP's update command only accepts
// the name fields — email and password are managed through their own flows.
export type UpdateTenantUserInput = {
	displayName: string | null;
	firstName: string;
	lastName: string;
};

export async function createTenantUser(
	tenantId: string,
	input: CreateTenantUserInput
): Promise<Result<TenantUserView>> {
	try {
		const data = await api.post<TenantUserView>(`/tenants/${tenantId}/users`, {
			email: input.email,
			display_name: input.displayName,
			first_name: input.firstName,
			last_name: input.lastName,
			is_email_verified: input.isEmailVerified,
			temporary_password: input.temporaryPassword
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function updateTenantUser(
	tenantId: string,
	userId: string,
	input: UpdateTenantUserInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/users/${userId}`, {
			display_name: input.displayName,
			first_name: input.firstName,
			last_name: input.lastName
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteTenantUser(tenantId: string, userId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/users/${userId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
