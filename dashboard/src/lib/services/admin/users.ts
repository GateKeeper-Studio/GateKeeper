import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's tenant-user list/get projection (snake_case — that projection
// has no camelCase rename). `display_name` is optional; the rest are always present.
export type TenantUserView = {
	user_id: string;
	display_name: string | null;
	first_name: string;
	last_name: string;
	email: string;
	// Tenant-defined custom field answers for this user, keyed by field_key.
	// Populated by the single-user GET only.
	custom_field_values?: Record<string, unknown> | null;
	created_at: string;
	updated_at: string | null;
	// Login methods attached to the account. Populated by the list endpoint so the
	// table can show that a native + social account are the same person; the
	// single-user GET may omit them.
	has_password?: boolean;
	providers?: string[];
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
		const data = await api.get<TenantUserView>(`/tenants/${tenantId}/users/${userId}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// What the create endpoint accepts (camelCase here, mapped to the IdP's snake_case
// Command below). No password: the IdP emails the user a single-use temporary one.
// `isEmailVerified` skips the confirmation email when true.
export type CreateTenantUserInput = {
	email: string;
	displayName: string | null;
	firstName: string;
	lastName: string;
	isEmailVerified: boolean;
};

// The created user, plus whether the welcome email with the temporary password went
// out (never the password itself).
export type CreatedTenantUser = TenantUserView & {
	temporary_password_expires_at: string;
	temporary_password_emailed: boolean;
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
): Promise<Result<CreatedTenantUser>> {
	try {
		const data = await api.post<CreatedTenantUser>(`/tenants/${tenantId}/users`, {
			email: input.email,
			display_name: input.displayName,
			first_name: input.firstName,
			last_name: input.lastName,
			is_email_verified: input.isEmailVerified
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

// How a user can sign in, as the IdP reports it to an administrator. Account-level
// facts: whether an app offers a method is that OAuth client's own setting. No
// secret is included (no password hash, TOTP seed or passkey key).
export type UserAuthenticationMethods = {
	emailVerified: boolean;
	password: { set: boolean; mustChange: boolean; changedAt: string | null };
	/** No password, a verified email and at least one passkey: signs in with the passkey alone. */
	passkeySignIn: boolean;
	/** A sign-in link by email can reach this account (its email is verified). */
	magicLink: boolean;
	socialAccounts: { provider: string; email: string | null; linkedAt: string }[];
	secondFactors: {
		/** "email" | "totp" | "webauthn"; null lets the login flow choose. */
		preferredMethod: string | null;
		emailCode: boolean;
		authenticatorApp: { confirmed: boolean; enrolledAt: string; lastUsedAt: string | null } | null;
		passkeys: { id: string; name: string | null; createdAt: string; lastUsedAt: string | null }[];
	};
};

// GET /tenants/:tenantId/users/:userId/authentication-methods
export async function getUserAuthenticationMethods(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<UserAuthenticationMethods>> {
	try {
		const data = await api.get<UserAuthenticationMethods>(
			`/tenants/${tenantId}/users/${userId}/authentication-methods`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export type PasswordResetResult = {
	userId: string;
	email: string;
	temporaryPasswordExpiresAt: string;
};

// POST /tenants/:tenantId/users/:userId/password-reset
// The IdP makes a temporary password and emails it to the user; the answer never
// carries it.
export async function resetUserPassword(
	tenantId: string,
	userId: string
): Promise<Result<PasswordResetResult>> {
	try {
		const data = await api.post<PasswordResetResult>(
			`/tenants/${tenantId}/users/${userId}/password-reset`,
			{}
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
