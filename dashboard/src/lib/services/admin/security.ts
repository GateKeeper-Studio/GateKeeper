import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';
import type { Paged } from './users';

// One login-attempt row, mirroring the API's LoginEventView.
export type LoginEventView = {
	id: string;
	tenant_id: string;
	user_id: string | null;
	client_id: string | null;
	username_normalized: string | null;
	ip_address: string | null;
	user_agent: string | null;
	country_code: string | null;
	region: string | null;
	city: string | null;
	latitude: number | null;
	longitude: number | null;
	asn: number | null;
	isp: string | null;
	credentials_valid: boolean;
	success: boolean;
	failure_reason: string | null;
	risk_score: number;
	risk_level: string;
	decision: string;
	created_at: string;
};

export type RiskSignalView = {
	signal_type: string;
	severity: string;
	points: number;
	metadata: unknown;
	explanation: string;
};

// The statistical model's verdict on one login (docs/RISK-SCORING.md). Present only
// when the platform runs hybrid scoring and the tenant is in shadow or hybrid mode.
export type RiskModelVerdict = {
	version: string;
	mode: 'shadow' | 'hybrid';
	probability: number;
	threshold: number;
	logit: number;
	intercept: number;
	would_escalate: boolean;
	applied: boolean;
	// Largest absolute contribution first; log-odds, positive raises the risk.
	contributions: { feature: string; value: number; contribution: number }[];
};

export type LoginEventLabelView = {
	label: 'legitimate' | 'fraudulent';
	source: 'admin' | 'mfa_passed' | 'mfa_failed';
	labeled_by: string | null;
	note: string | null;
	updated_at: string;
};

export type LoginEventDetail = {
	event: LoginEventView;
	signals: RiskSignalView[];
	risk_model: RiskModelVerdict | null;
	features: { v: number; values: Record<string, number> } | null;
	label: LoginEventLabelView | null;
	related_by_ip: LoginEventView[];
	related_by_user: LoginEventView[];
};

export type SecurityMetrics = {
	total_attempts: number;
	successful_logins: number;
	failed_logins: number;
	blocked_attempts: number;
	high_risk_attempts: number;
	critical_risk_attempts: number;
	password_spraying_events: number;
};

export type SuspiciousIpView = {
	ip_address: string;
	attempts: number;
	failed_attempts: number;
	distinct_users: number;
	country_code: string | null;
	max_risk_score: number;
	first_seen: string;
	last_seen: string;
	blocked: boolean;
	block_id: string | null;
};

export type CountryCountView = { country_code: string | null; count: number };

export type SecurityOverview = {
	from: string;
	to: string;
	metrics: SecurityMetrics;
	top_suspicious_ips: SuspiciousIpView[];
	logins_by_country: CountryCountView[];
};

export type AttackedUserView = {
	user_id: string;
	username: string | null;
	failed_attempts: number;
	distinct_ips: number;
	last_failed_at: string | null;
	last_success_at: string | null;
};

export type RiskPolicyView = {
	tenant_id: string;
	enabled: boolean;
	failed_ip_threshold: number;
	failed_user_threshold: number;
	password_spraying_user_threshold: number;
	tenant_attack_threshold: number;
	medium_risk_threshold: number;
	high_risk_threshold: number;
	critical_risk_threshold: number;
	require_mfa_on_high_risk: boolean;
	block_on_critical_risk: boolean;
	temporary_block_minutes: number;
	// Whether the statistical score applies to this tenant (docs/RISK-SCORING.md).
	scoring_mode: 'rules' | 'shadow' | 'hybrid';
};

export type SecurityBlockView = {
	id: string;
	tenant_id: string;
	block_type: string;
	ip_address: string | null;
	user_id: string | null;
	reason: string;
	source: string;
	expires_at: string;
	created_at: string;
};

export type RangeParams = { from?: string; to?: string; limit?: number };

export type ListLoginEventParams = {
	page?: number;
	pageSize?: number;
	userId?: string;
	ipAddress?: string;
	countryCode?: string;
	riskLevel?: string;
	decision?: string;
	success?: boolean;
	signalType?: string;
	from?: string;
	to?: string;
};

const base = (tenantId: string) => `/tenants/${tenantId}/security`;

// GET /tenants/{tenantId}/security/overview
export async function getSecurityOverview(
	tenantId: string,
	params: RangeParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<SecurityOverview>> {
	const q = new URLSearchParams();
	if (params.from) q.set('from', params.from);
	if (params.to) q.set('to', params.to);
	try {
		const data = await api.get<SecurityOverview>(`${base(tenantId)}/overview?${q}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/security/login-events
export async function listLoginEvents(
	tenantId: string,
	params: ListLoginEventParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<LoginEventView>>> {
	const q = new URLSearchParams();
	q.set('page', String(params.page ?? 1));
	q.set('page_size', String(params.pageSize ?? 25));
	if (params.userId) q.set('user_id', params.userId);
	if (params.ipAddress) q.set('ip_address', params.ipAddress);
	if (params.countryCode) q.set('country_code', params.countryCode);
	if (params.riskLevel) q.set('risk_level', params.riskLevel);
	if (params.decision) q.set('decision', params.decision);
	if (params.success !== undefined) q.set('success', String(params.success));
	if (params.signalType) q.set('signal_type', params.signalType);
	if (params.from) q.set('from', params.from);
	if (params.to) q.set('to', params.to);
	try {
		const data = await api.get<Paged<LoginEventView>>(
			`${base(tenantId)}/login-events?${q}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/security/login-events/{id}
export async function getLoginEvent(
	tenantId: string,
	id: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<LoginEventDetail>> {
	try {
		const data = await api.get<LoginEventDetail>(`${base(tenantId)}/login-events/${id}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/security/suspicious-ips
export async function listSuspiciousIps(
	tenantId: string,
	params: RangeParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<{ items: SuspiciousIpView[] }>> {
	const q = new URLSearchParams();
	if (params.from) q.set('from', params.from);
	if (params.to) q.set('to', params.to);
	if (params.limit) q.set('limit', String(params.limit));
	try {
		const data = await api.get<{ items: SuspiciousIpView[] }>(
			`${base(tenantId)}/suspicious-ips?${q}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/security/attacked-users
export async function listAttackedUsers(
	tenantId: string,
	params: RangeParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<{ items: AttackedUserView[] }>> {
	const q = new URLSearchParams();
	if (params.from) q.set('from', params.from);
	if (params.to) q.set('to', params.to);
	if (params.limit) q.set('limit', String(params.limit));
	try {
		const data = await api.get<{ items: AttackedUserView[] }>(
			`${base(tenantId)}/attacked-users?${q}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/security/risk-policy
export async function getRiskPolicy(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<RiskPolicyView>> {
	try {
		const data = await api.get<RiskPolicyView>(`${base(tenantId)}/risk-policy`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /tenants/{tenantId}/security/risk-policy
export async function updateRiskPolicy(
	tenantId: string,
	body: Omit<RiskPolicyView, 'tenant_id'>,
	fetchFn: typeof fetch = fetch
): Promise<Result<RiskPolicyView>> {
	try {
		const data = await api.put<RiskPolicyView>(`${base(tenantId)}/risk-policy`, body, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /tenants/{tenantId}/security/blocks/ip
export async function blockIp(
	tenantId: string,
	body: { ip_address: string; reason?: string; minutes?: number },
	fetchFn: typeof fetch = fetch
): Promise<Result<SecurityBlockView>> {
	try {
		const data = await api.post<SecurityBlockView>(`${base(tenantId)}/blocks/ip`, body, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/security/blocks/ip/{blockId}
export async function unblockIp(
	tenantId: string,
	blockId: string,
	fetchFn: typeof fetch = fetch
): Promise<VoidResult> {
	try {
		await api.delete<void>(`${base(tenantId)}/blocks/ip/${blockId}`, fetchFn);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// PUT /tenants/{tenantId}/security/login-events/{id}/label
// An admin's verdict, the statistical model's training feedback.
export async function labelLoginEvent(
	tenantId: string,
	id: string,
	body: { label: 'legitimate' | 'fraudulent'; note?: string },
	fetchFn: typeof fetch = fetch
): Promise<Result<LoginEventLabelView>> {
	try {
		const data = await api.put<LoginEventLabelView>(
			`${base(tenantId)}/login-events/${id}/label`,
			body,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/security/login-events/{id}/label
export async function clearLoginEventLabel(
	tenantId: string,
	id: string,
	fetchFn: typeof fetch = fetch
): Promise<VoidResult> {
	try {
		await api.delete<void>(`${base(tenantId)}/login-events/${id}/label`, fetchFn);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
