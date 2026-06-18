import { api } from '../base/api';
import { ApiError, type Result } from '../types';

// ── Data-subject export (WI-05) ──────────────────────────────────────────────
export type UserDataExport = {
	generated_at: string;
	user: Record<string, unknown>;
	consents: unknown[];
	roles: string[];
	groups: string[];
	policy_acceptances: unknown[];
};

// GET /tenants/{tenantId}/users/{userId}/data-export
export async function exportUserData(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<UserDataExport>> {
	try {
		const data = await api.get<UserDataExport>(
			`/tenants/${tenantId}/users/${userId}/data-export`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// ── Right to erasure / anonymization (WI-06) ─────────────────────────────────
export type EraseResult = { user_id: string; anonymized: boolean };

// POST /tenants/{tenantId}/users/{userId}/erase
export async function eraseUser(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<EraseResult>> {
	try {
		const data = await api.post<EraseResult>(
			`/tenants/${tenantId}/users/${userId}/erase`,
			undefined,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// ── Policy / ToS acceptance (WI-04) ──────────────────────────────────────────
export type PolicyAcceptanceView = {
	document_type: string;
	document_version: string;
	accepted_at: string;
};

// GET /tenants/{tenantId}/users/{userId}/policy-acceptances
export async function listUserAcceptances(
	tenantId: string,
	userId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<PolicyAcceptanceView[]>> {
	try {
		const data = await api.get<PolicyAcceptanceView[]>(
			`/tenants/${tenantId}/users/${userId}/policy-acceptances`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /tenants/{tenantId}/users/{userId}/policy-acceptances
export async function recordPolicyAcceptance(
	tenantId: string,
	userId: string,
	documentType: string,
	documentVersion: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<PolicyAcceptanceView>> {
	try {
		const data = await api.post<PolicyAcceptanceView>(
			`/tenants/${tenantId}/users/${userId}/policy-acceptances`,
			{ document_type: documentType, document_version: documentVersion },
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// ── Retention purge (WI-03) ──────────────────────────────────────────────────
export type PurgeReport = {
	refresh_tokens: number;
	auth_sessions: number;
	authorization_codes: number;
	total: number;
};

// POST /admin/maintenance/purge
export async function runPurge(fetchFn: typeof fetch = fetch): Promise<Result<PurgeReport>> {
	try {
		const data = await api.post<PurgeReport>('/admin/maintenance/purge', undefined, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
