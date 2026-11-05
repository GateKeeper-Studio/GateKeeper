import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result } from '../types';

// Mirrors the IdP's email-template list projection (camelCase). `isCustomized` is
// false when the tenant is showing the built-in default.
export type EmailTemplateSummary = {
	templateType: string;
	label: string;
	description: string;
	variables: string[];
	updatedAt: string | null;
	isCustomized: boolean;
};

// The full template (custom or default). `bodyMjml` is the editable MJML source.
export type EmailTemplateView = {
	templateType: string;
	subject: string;
	bodyMjml: string;
	variables: string[];
	updatedAt: string | null;
	isCustomized: boolean;
};

export type EmailTemplatePreview = {
	subject: string;
	html: string;
};

// GET /tenants/{tenantId}/email-templates
export async function listEmailTemplates(
	tenantId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<{ items: EmailTemplateSummary[] }>> {
	try {
		const data = await api.get<{ items: EmailTemplateSummary[] }>(
			`/tenants/${tenantId}/email-templates`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// GET /tenants/{tenantId}/email-templates/{type}
export async function getEmailTemplate(
	tenantId: string,
	type: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<EmailTemplateView>> {
	try {
		const data = await api.get<EmailTemplateView>(
			`/tenants/${tenantId}/email-templates/${type}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /tenants/{tenantId}/email-templates/{type}
export async function updateEmailTemplate(
	tenantId: string,
	type: string,
	subject: string,
	bodyMjml: string
): Promise<Result<EmailTemplateView>> {
	try {
		const data = await api.put<EmailTemplateView>(`/tenants/${tenantId}/email-templates/${type}`, {
			subject,
			body_mjml: bodyMjml
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/email-templates/{type} — resets to the built-in
// default and returns it so the editor can refresh immediately.
export async function resetEmailTemplate(
	tenantId: string,
	type: string
): Promise<Result<EmailTemplateView>> {
	try {
		const data = await api.delete<EmailTemplateView>(
			`/tenants/${tenantId}/email-templates/${type}`
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// POST /tenants/{tenantId}/email-templates/{type}/preview — renders the *draft*
// (server-side, same pipeline as a real send) so the preview matches what's sent.
export async function previewEmailTemplate(
	tenantId: string,
	type: string,
	subject: string,
	bodyMjml: string
): Promise<Result<EmailTemplatePreview>> {
	try {
		const data = await api.post<EmailTemplatePreview>(
			`/tenants/${tenantId}/email-templates/${type}/preview`,
			{ subject, body_mjml: bodyMjml }
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
