import { api } from '../base/api';
import { ApiError, type Result, type VoidResult } from '../types';
import type { Paged } from './users';
export type { Paged };

export type TenantGroupView = {
	group_id: string;
	name: string;
	description: string | null;
	member_count: number;
	created_at: string;
	updated_at: string | null;
};

export type ListGroupsParams = { page?: number; pageSize?: number };

export async function listTenantGroups(
	tenantId: string,
	params: ListGroupsParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<TenantGroupView>>> {
	const page = params.page ?? 1;
	const pageSize = params.pageSize ?? 100;
	try {
		const data = await api.get<Paged<TenantGroupView>>(
			`/tenants/${tenantId}/groups?page=${page}&page_size=${pageSize}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function getTenantGroup(
	tenantId: string,
	groupId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<TenantGroupView>> {
	try {
		const data = await api.get<TenantGroupView>(
			`/tenants/${tenantId}/groups/${groupId}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export type CreateTenantGroupInput = {
	name: string;
	description: string | null;
};

export type UpdateTenantGroupInput = {
	name: string;
	description: string | null;
};

export async function createTenantGroup(
	tenantId: string,
	input: CreateTenantGroupInput
): Promise<Result<TenantGroupView>> {
	try {
		const data = await api.post<TenantGroupView>(`/tenants/${tenantId}/groups`, {
			name: input.name,
			description: input.description
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function updateTenantGroup(
	tenantId: string,
	groupId: string,
	input: UpdateTenantGroupInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/groups/${groupId}`, {
			name: input.name,
			description: input.description
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteTenantGroup(tenantId: string, groupId: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/groups/${groupId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function listGroupMembers(
	tenantId: string,
	groupId: string,
	params: ListGroupsParams = {}
): Promise<Result<Paged<import('./users').TenantUserView>>> {
	const page = params.page ?? 1;
	const pageSize = params.pageSize ?? 100;
	try {
		const data = await api.get<Paged<import('./users').TenantUserView>>(
			`/tenants/${tenantId}/groups/${groupId}/members?page=${page}&page_size=${pageSize}`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function addGroupMembers(
	tenantId: string,
	groupId: string,
	userIds: string[]
): Promise<Result<TenantGroupView>> {
	try {
		const data = await api.post<TenantGroupView>(
			`/tenants/${tenantId}/groups/${groupId}/members`,
			{ user_ids: userIds }
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function removeGroupMember(
	tenantId: string,
	groupId: string,
	userId: string
): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/groups/${groupId}/members/${userId}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function listUserGroups(
	tenantId: string,
	userId: string
): Promise<Result<TenantGroupView[]>> {
	try {
		const data = await api.get<TenantGroupView[]>(
			`/tenants/${tenantId}/users/${userId}/groups`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
