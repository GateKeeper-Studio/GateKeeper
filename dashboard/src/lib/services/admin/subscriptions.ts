import { api } from '../base/api';
import { ApiError, type Result } from '../types';

// The plan a subscription is bound to (subset of the catalog plan, embedded in the
// billing overview).
export type BillingPlanView = {
	planId: string;
	name: string;
	description: string | null;
	priceCents: number;
	billingInterval: string;
	maxTenants: number;
	maxUsers: number;
	isActive: boolean;
};

// Current usage measured against the plan's limits — drives the usage bars and the
// client-side guard that disables plan downgrades the usage cannot fit.
export type BillingUsageView = {
	tenants: number;
	maxTenants: number;
	maxUsersInAnyTenant: number;
	maxUsers: number;
};

// GET /admin/me/organization/subscription — subscription + plan + usage.
export type BillingOverview = {
	subscriptionId: string;
	organizationId: string;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
	plan: BillingPlanView;
	usage: BillingUsageView;
};

// Contracting a subscription also returns the auto-provisioned default ("master") tenant.
export type CreatedSubscription = {
	subscriptionId: string;
	organizationId: string;
	planId: string;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
	defaultTenantId: string;
};

// GET /admin/me/organization/subscription — the org's billing overview, or `null`
// when the organization has not contracted a subscription yet (404).
export async function getMyBilling(
	fetchFn: typeof fetch = fetch
): Promise<Result<BillingOverview | null>> {
	try {
		const data = await api.get<BillingOverview>('/admin/me/organization/subscription', fetchFn);
		return [data, null];
	} catch (error) {
		const apiError = error as ApiError;
		if (apiError.status === 404) return [null, null];
		return [null, apiError];
	}
}

// POST /admin/me/organization/subscription — contract a subscription (org has none
// yet). Provisions the default "master" tenant.
export async function contractSubscription(planId: string): Promise<Result<CreatedSubscription>> {
	try {
		const data = await api.post<CreatedSubscription>('/admin/me/organization/subscription', {
			plan_id: planId
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /admin/me/organization/subscription — change plan and/or active state. The
// server blocks a downgrade the current usage cannot fit (409).
export async function changePlan(
	planId: string,
	isActive: boolean
): Promise<Result<BillingOverview>> {
	try {
		const data = await api.put<BillingOverview>('/admin/me/organization/subscription', {
			plan_id: planId,
			is_active: isActive
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
