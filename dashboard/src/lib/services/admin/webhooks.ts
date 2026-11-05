import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result, type VoidResult } from '../types';

// The IdP's SubscriptionView / DeliveryView serialize camelCase. Request
// bodies (create/update Command) are snake_case — that struct has no rename.
export type WebhookSubscriptionView = {
	id: string;
	name: string;
	url: string;
	// Empty means "every event type".
	eventTypes: string[];
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

// The create response flattens the view and adds the raw signing secret EXACTLY
// ONCE. It is stored encrypted and is never retrievable again.
export type CreatedWebhookSubscription = WebhookSubscriptionView & { secret: string };

export type WebhookDeliveryStatus = 'pending' | 'delivered' | 'dead';

export type WebhookDeliveryView = {
	id: string;
	eventId: string;
	subscriptionId: string;
	status: WebhookDeliveryStatus;
	attempts: number;
	nextAttemptAt: string;
	lastError: string | null;
	lastStatus: number | null;
	createdAt: string;
	deliveredAt: string | null;
};

export type WebhookTestResult = {
	success: boolean;
	status: number | null;
	error: string | null;
};

export type WebhookSubscriptionInput = {
	name: string;
	url: string;
	eventTypes: string[];
};

// Mirrors the IdP's DomainEventType — the only values it accepts.
export const WEBHOOK_EVENT_TYPES = ['user.created', 'user.updated', 'user.deleted'] as const;

type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

export async function listWebhookSubscriptions(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<WebhookSubscriptionView>>> {
	try {
		const data = await api.get<Paged<WebhookSubscriptionView>>(
			`/tenants/${tenantId}/webhooks?page=1&page_size=100`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function createWebhookSubscription(
	tenantId: string,
	input: WebhookSubscriptionInput
): Promise<Result<CreatedWebhookSubscription>> {
	try {
		const data = await api.post<CreatedWebhookSubscription>(`/tenants/${tenantId}/webhooks`, {
			name: input.name,
			url: input.url,
			event_types: input.eventTypes,
			is_active: true
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT replaces every editable field, so the caller passes the full current
// subscription with only the field it means to change altered.
export async function setWebhookSubscriptionActive(
	tenantId: string,
	subscription: WebhookSubscriptionView,
	isActive: boolean
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/webhooks/${subscription.id}`, {
			name: subscription.name,
			url: subscription.url,
			event_types: subscription.eventTypes,
			is_active: isActive
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function updateWebhookSubscription(
	tenantId: string,
	subscription: WebhookSubscriptionView,
	input: WebhookSubscriptionInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/webhooks/${subscription.id}`, {
			name: input.name,
			url: input.url,
			event_types: input.eventTypes,
			is_active: subscription.isActive
		});
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteWebhookSubscription(tenantId: string, id: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/webhooks/${id}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function sendWebhookTestEvent(
	tenantId: string,
	id: string
): Promise<Result<WebhookTestResult>> {
	try {
		const data = await api.post<WebhookTestResult>(`/tenants/${tenantId}/webhooks/${id}/test`);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function listWebhookDeliveries(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<WebhookDeliveryView>>> {
	try {
		const data = await api.get<Paged<WebhookDeliveryView>>(
			`/tenants/${tenantId}/webhook-deliveries?page=1&page_size=25`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
