import { api } from '../base/api';
import { ApiError, type Result, type VoidResult } from '../types';
import type { Paged } from './users';
export type { Paged };

export type TenantRoleView = {
	role_id: string;
	name: string;
	description: string | null;
	user_count: number;
	created_at: string;
	updated_at: string | null;
};

export type ListRolesParams = { page?: number; pageSize?: number };

export async function listTenantRoles(
	tenantId: string,
	params: ListRolesParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<TenantRoleView>>> {
	const page = params.page ?? 1;
	const pageSize = params.pageSize ?? 100;
	try {
		const data = await api.get<Paged<TenantRoleView>>(
			`/tenants/${tenantId}/roles?page=${page}&page_size=${pageSize}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function getTenantRole(
	tenantId: string,
	roleId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<TenantRoleView>> {
	try {
		const data = await api.get<TenantRoleView>(
			`/tenants/${tenantId}/roles/${roleId}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export type CreateTenantRoleInput = {
	name: string;
	description: string | null;
};

export type UpdateTenantRoleInput = {
	name: string;
	description: string | null;
};

export async function createTenantRole(
	tenantId: string,
	input: CreateTenantRoleInput
): Promise<Result<TenantRoleView>> {
	try {
		const data = await api.post<TenantRoleView>(`/tenants/${tenantId}/roles`, {
			name: input.name,
			description: input.description
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function updateTenantRole(
	tenantId: string,
	roleId: string,
	input: UpdateTenantRoleInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/roles/${roleId}`, {
			name: input.name,
			description: input.description
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteTenantRole(tenantId: string, roleId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/roles/${roleId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function listRoleUsers(
	tenantId: string,
	roleId: string,
	params: ListRolesParams = {}
): Promise<Result<Paged<import('./users').TenantUserView>>> {
	const page = params.page ?? 1;
	const pageSize = params.pageSize ?? 100;
	try {
		const data = await api.get<Paged<import('./users').TenantUserView>>(
			`/tenants/${tenantId}/roles/${roleId}/users?page=${page}&page_size=${pageSize}`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function assignRoleUsers(
	tenantId: string,
	roleId: string,
	userIds: string[]
): Promise<Result<{ assigned: number; user_count: number }>> {
	try {
		const data = await api.post<{ assigned: number; user_count: number }>(
			`/tenants/${tenantId}/roles/${roleId}/users`,
			{ user_ids: userIds }
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function unassignRoleUser(
	tenantId: string,
	roleId: string,
	userId: string
): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/roles/${roleId}/users/${userId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function listUserRoles(
	tenantId: string,
	userId: string
): Promise<Result<TenantRoleView[]>> {
	try {
		const data = await api.get<TenantRoleView[]>(
			`/tenants/${tenantId}/users/${userId}/roles`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
