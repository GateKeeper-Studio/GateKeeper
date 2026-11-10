import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

// The admins of the signed-in admin's organization (`/admin/organization/*`). Every
// change needs the acting admin's current password; the server audits each one.

export type OrganizationAdmin = {
	id: string;
	email: string;
	name: string;
	isActive: boolean;
	mfaEnabled: boolean;
	/** Set while the account is locked after too many failed sign-ins. */
	lockedUntil: string | null;
	lastSignInAt: string | null;
	createdAt: string;
};

export type AdminInvitation = {
	id: string;
	email: string;
	invitedBy: string | null;
	expiresAt: string;
	createdAt: string;
};

/** The invitation link is returned once, at creation; only its hash is kept. */
export type CreatedInvitation = {
	id: string;
	email: string;
	expiresAt: string;
	url: string;
};

async function attempt<T>(call: () => Promise<T>): Promise<Result<T>> {
	try {
		return [await call(), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

async function attemptVoid(call: () => Promise<unknown>): Promise<VoidResult> {
	try {
		await call();
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export const listOrganizationAdmins = (fetchFn: typeof fetch = fetch) =>
	attempt(
		async () =>
			(await api.get<{ admins: OrganizationAdmin[] }>('/admin/organization/admins', fetchFn)).admins
	);

export const deactivateAdmin = (adminId: string, currentPassword: string) =>
	attemptVoid(() =>
		api.post(`/admin/organization/admins/${adminId}/deactivate`, {
			current_password: currentPassword
		})
	);

export const reactivateAdmin = (adminId: string, currentPassword: string) =>
	attemptVoid(() =>
		api.post(`/admin/organization/admins/${adminId}/reactivate`, {
			current_password: currentPassword
		})
	);

export const resetAdminMfa = (adminId: string, currentPassword: string) =>
	attemptVoid(() =>
		api.deleteWith(`/admin/organization/admins/${adminId}/mfa`, {
			current_password: currentPassword
		})
	);

export const listInvitations = (fetchFn: typeof fetch = fetch) =>
	attempt(
		async () =>
			(
				await api.get<{ invitations: AdminInvitation[] }>(
					'/admin/organization/invitations',
					fetchFn
				)
			).invitations
	);

export const inviteAdmin = (email: string, currentPassword: string) =>
	attempt(() =>
		api.post<CreatedInvitation>('/admin/organization/invitations', {
			email,
			current_password: currentPassword
		})
	);

export const revokeInvitation = (invitationId: string) =>
	attemptVoid(() => api.delete(`/admin/organization/invitations/${invitationId}`));

export const setAdminMfaPolicy = (required: boolean, currentPassword: string) =>
	attempt(() =>
		api.put<{ requireAdminMfa: boolean }>('/admin/organization/mfa-policy', {
			required,
			current_password: currentPassword
		})
	);
