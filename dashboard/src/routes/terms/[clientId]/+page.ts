import type { PageLoad } from './$types';
import { getPublicLegalDocument } from '$lib/services/auth/get-legal-document';

// Public Terms of Service for a given client. The backend resolves the client's
// tenant and returns that tenant's document or the built-in default.
export const load: PageLoad = async ({ params, fetch }) => {
	const [doc, error] = await getPublicLegalDocument(params.clientId, 'terms', fetch);

	return {
		content: doc?.content ?? '',
		updatedAt: doc?.updatedAt ?? null,
		loadError: error ? { message: error.message } : null
	};
};
