import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's admin profile Response (camelCase projection). `name` may be empty
// for accounts bootstrapped before a name was set; the UI falls back to the email.
export type AdminProfileView = {
	id: string;
	organizationId: string;
	email: string;
	name: string;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

export type AdminProfileInput = {
	name: string;
	email: string;
};

export type ChangePasswordInput = {
	currentPassword: string;
	newPassword: string;
};

// GET /admin/me/profile — the signed-in admin's own account.
export async function getMyProfile(
	fetchFn: typeof fetch = fetch
): Promise<Result<AdminProfileView>> {
	try {
		const data = await api.get<AdminProfileView>('/admin/me/profile', fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /admin/me/profile — update the admin's display name and login email.
export async function updateMyProfile(input: AdminProfileInput): Promise<Result<AdminProfileView>> {
	try {
		const data = await api.put<AdminProfileView>('/admin/me/profile', {
			name: input.name,
			email: input.email
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /admin/me/password — change the password (the current one re-authenticates).
export async function changeMyPassword(input: ChangePasswordInput): Promise<VoidResult> {
	try {
		await api.put<void>('/admin/me/password', {
			current_password: input.currentPassword,
			new_password: input.newPassword
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// DELETE /admin/me — LGPD soft-deactivation: disables the account and ends every
// session. Reversible by another admin; the server refuses if this is the org's last
// active admin.
export async function deactivateMyAccount(): Promise<VoidResult> {
	try {
		await api.delete<void>('/admin/me');
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
