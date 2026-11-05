import { adminApi as api } from '../base/admin-api';
import { ApiError, type Result } from '../types';

/** URL slug identifying a legal document. */
export type LegalDocType = 'terms' | 'privacy';

// Mirrors the IdP's legal-document Response (camelCase projection). `content` is
// markdown; `isCustomized` is false when the built-in default is being served
// (in which case `updatedAt` is null).
export type LegalDocumentView = {
	documentType: string;
	content: string;
	updatedAt: string | null;
	isCustomized: boolean;
};

// GET /tenants/{tenantId}/legal/{type}
export async function getLegalDocument(
	tenantId: string,
	type: LegalDocType,
	fetchFn: typeof fetch = fetch
): Promise<Result<LegalDocumentView>> {
	try {
		const data = await api.get<LegalDocumentView>(`/tenants/${tenantId}/legal/${type}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// PUT /tenants/{tenantId}/legal/{type}
export async function updateLegalDocument(
	tenantId: string,
	type: LegalDocType,
	markdown: string
): Promise<Result<LegalDocumentView>> {
	try {
		const data = await api.put<LegalDocumentView>(`/tenants/${tenantId}/legal/${type}`, {
			markdown_content: markdown
		});
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

// DELETE /tenants/{tenantId}/legal/{type} — resets to the built-in default and
// returns it so the editor can refresh immediately.
export async function resetLegalDocument(
	tenantId: string,
	type: LegalDocType
): Promise<Result<LegalDocumentView>> {
	try {
		const data = await api.delete<LegalDocumentView>(`/tenants/${tenantId}/legal/${type}`);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
