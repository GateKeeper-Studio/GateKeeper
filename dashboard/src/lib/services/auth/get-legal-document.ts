import { api } from '../base/api';
import { ApiError, type Result } from '../types';
import type { LegalDocType, LegalDocumentView } from '../admin/legal';

/**
 * Public legal document for a given client. The backend resolves the client's
 * tenant (the public surface never sees the tenant UUID) and returns that
 * tenant's document or the built-in default. Pass `event.fetch` from `load`.
 */
export async function getPublicLegalDocument(
	clientId: string,
	type: LegalDocType,
	fetchFn: typeof fetch = fetch
): Promise<Result<LegalDocumentView>> {
	try {
		const data = await api.get<LegalDocumentView>(
			`/v1/auth/client/${clientId}/legal/${type}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

/** The platform default legal document, with no tenant/client context. */
export async function getDefaultLegalDocument(
	type: LegalDocType,
	fetchFn: typeof fetch = fetch
): Promise<Result<LegalDocumentView>> {
	try {
		const data = await api.get<LegalDocumentView>(`/v1/legal/${type}`, fetchFn);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
