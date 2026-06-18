import { api } from '../base/api';
import { ApiError, type Result } from '../types';

// Mirrors the IdP's subscription-plan list Response (camelCase projection).
// Read-only here: the subscription form uses this to populate its plan picker.
export type SubscriptionPlanView = {
	planId: string;
	name: string;
	description: string | null;
	priceCents: number;
	billingInterval: string;
	maxTenants: number;
	maxUsers: number;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

// Formats `priceCents` as a localized USD amount (e.g. 1900 → "$19.00").
export function formatPrice(priceCents: number): string {
	return (priceCents / 100).toLocaleString(undefined, {
		style: 'currency',
		currency: 'USD'
	});
}

// Formats a plan into a "Name — $19.00/monthly" label for compact pickers.
export function planLabel(plan: {
	name: string;
	priceCents: number;
	billingInterval: string;
}): string {
	return `${plan.name} — ${formatPrice(plan.priceCents)}/${plan.billingInterval}`;
}

// GET /subscription-plans — large first page for the form's plan select.
export async function listSubscriptionPlans(
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<SubscriptionPlanView>>> {
	try {
		const data = await api.get<Paged<SubscriptionPlanView>>(
			'/subscription-plans?page=1&page_size=100',
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
