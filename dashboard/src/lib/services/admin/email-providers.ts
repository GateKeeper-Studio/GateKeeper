import { api } from '../base/api';
import { ApiError, type Result, type VoidResult } from '../types';

// Mirrors the IdP's ProviderView (camelCase). The password is never returned.
export type EmailProvider = {
	id: string;
	name: string;
	host: string;
	port: number;
	username: string;
	fromEmail: string;
	fromName: string | null;
	encryption: string;
	isDefault: boolean;
	isActive: boolean;
	createdAt: string;
	updatedAt: string | null;
};

// The IdP's PagedContent wrapper (snake_case `page_size`).
type Paged<T> = { items: T[]; total: number; page: number; page_size: number };

export type EmailProviderInput = {
	name: string;
	host: string;
	port: number;
	username: string;
	password?: string; // required on create; optional on update (kept if omitted)
	fromEmail: string;
	fromName?: string | null;
	encryption: string; // 'starttls' | 'tls' | 'none'
	isDefault: boolean;
	isActive?: boolean;
};

export async function listEmailProviders(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<Paged<EmailProvider>>> {
	try {
		const data = await api.get<Paged<EmailProvider>>(
			`/tenants/${tenantId}/email-providers?page=1&page_size=100`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function createEmailProvider(
	tenantId: string,
	input: EmailProviderInput
): Promise<VoidResult> {
	try {
		await api.post(`/tenants/${tenantId}/email-providers`, input);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function updateEmailProvider(
	tenantId: string,
	id: string,
	input: EmailProviderInput
): Promise<VoidResult> {
	try {
		await api.put(`/tenants/${tenantId}/email-providers/${id}`, input);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

export async function deleteEmailProvider(tenantId: string, id: string): Promise<VoidResult> {
	try {
		await api.delete(`/tenants/${tenantId}/email-providers/${id}`);
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}

// Sends a one-off test email through the provider so the admin can verify its
// SMTP credentials. On failure the ApiError carries the transport's exact
// message (e.g. an SMTP 535 authentication failure).
export async function sendTestEmail(
	tenantId: string,
	id: string,
	to: string
): Promise<VoidResult> {
	try {
		await api.post(`/tenants/${tenantId}/email-providers/${id}/test`, { to });
		return [null];
	} catch (error) {
		return [error as ApiError];
	}
}
