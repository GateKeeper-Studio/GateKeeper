import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result } from '../types';
import type { Paged } from './users';

// One audit-log row, mirroring the API's list_audit_events response shape.
export type AuditEventView = {
	id: string;
	tenant_id: string | null;
	actor_type: string;
	actor_id: string | null;
	action: string;
	resource_type: string | null;
	resource_id: string | null;
	outcome: string;
	ip_address: string | null;
	user_agent: string | null;
	metadata: unknown;
	created_at: string;
};

export type ListAuditParams = {
	page?: number;
	pageSize?: number;
	action?: string;
	actorId?: string;
	outcome?: string;
	from?: string;
	to?: string;
};

// GET /tenants/{tenantId}/audit-events — the append-only security/audit log (WI-02).
export async function listAuditEvents(
	tenantId: string,
	params: ListAuditParams = {},
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<AuditEventView>>> {
	const q = new URLSearchParams();
	q.set('page', String(params.page ?? 1));
	q.set('page_size', String(params.pageSize ?? 25));
	if (params.action) q.set('action', params.action);
	if (params.actorId) q.set('actor_id', params.actorId);
	if (params.outcome) q.set('outcome', params.outcome);
	if (params.from) q.set('from', params.from);
	if (params.to) q.set('to', params.to);

	try {
		const data = await api.get<Paged<AuditEventView>>(
			`/tenants/${tenantId}/audit-events?${q.toString()}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
