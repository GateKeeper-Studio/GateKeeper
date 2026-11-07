import type { PageLoad } from './$types';
import { getLegalDocument } from '$lib/services/admin/legal';

// Loads the tenant's Terms of Service and Privacy Policy (the built-in default is
// returned when a document has not been customized). Errors are returned (not
// thrown) so the page can render an alert.
export const load: PageLoad = async ({ params, fetch }) => {
	const [terms, termsError] = await getLegalDocument(params.tenantId, 'terms', fetch);
	const [privacy, privacyError] = await getLegalDocument(params.tenantId, 'privacy', fetch);

	const error = termsError ?? privacyError;

	return {
		tenantId: params.tenantId,
		terms: terms ?? null,
		privacy: privacy ?? null,
		loadError: error ? { message: error.message } : null
	};
};
