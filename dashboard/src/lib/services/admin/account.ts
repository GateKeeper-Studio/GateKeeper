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
	// Required by the server only when the email changes (it is the sign-in id).
	currentPassword?: string;
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
			email: input.email,
			current_password: input.currentPassword || undefined
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

// PUT /admin/me/console-tour — the admin finished or closed that version of the
// console's guided tour, so first access in another browser skips it.
export async function recordConsoleTour(version: number): Promise<VoidResult> {
	try {
		await api.put<void>('/admin/me/console-tour', { version });
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// DELETE /admin/me — LGPD soft-deactivation: disables the account and ends every
// session. Needs the current password; the server refuses if this is the org's last
// active admin.
export async function deactivateMyAccount(currentPassword: string): Promise<VoidResult> {
	try {
		await api.deleteWith<void>('/admin/me', { current_password: currentPassword });
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// ── Two-step verification ───────────────────────────────────────────────────────

export type MfaStatus = {
	totpEnabled: boolean;
	enabledAt: string | null;
	recoveryCodesRemaining: number;
};

export type TotpEnrollment = {
	secretBase32: string;
	otpauthUrl: string;
};

export type RecoveryCodes = { recoveryCodes: string[] };

// GET /admin/me/mfa — whether sign-in asks for a code.
export async function getMyMfa(fetchFn: typeof fetch = fetch): Promise<Result<MfaStatus>> {
	try {
		return [await api.get<MfaStatus>('/admin/me/mfa', fetchFn), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /admin/me/mfa/totp — a fresh seed for the authenticator app (not in force
// until confirmed).
export async function startTotp(currentPassword: string): Promise<Result<TotpEnrollment>> {
	try {
		const data = await api.post<TotpEnrollment>('/admin/me/mfa/totp', {
			current_password: currentPassword
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /admin/me/mfa/totp/confirm — a code from the app turns it on; returns the
// recovery codes, shown once.
export async function confirmTotp(code: string): Promise<Result<RecoveryCodes>> {
	try {
		return [await api.post<RecoveryCodes>('/admin/me/mfa/totp/confirm', { code }), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /admin/me/mfa/recovery-codes — replaces every recovery code.
export async function regenerateRecoveryCodes(
	currentPassword: string
): Promise<Result<RecoveryCodes>> {
	try {
		const data = await api.post<RecoveryCodes>('/admin/me/mfa/recovery-codes', {
			current_password: currentPassword
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /admin/me/mfa — turns it off; needs the password and a code.
export async function disableMfa(currentPassword: string, code: string): Promise<VoidResult> {
	try {
		await api.deleteWith<void>('/admin/me/mfa', { current_password: currentPassword, code });
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
