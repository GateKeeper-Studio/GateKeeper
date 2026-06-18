import type { PageLoad } from './$types';
import { getDefaultLegalDocument } from '$lib/services/auth/get-legal-document';

// The bare /terms route has no client/tenant context, so it shows the platform
// default Terms of Service.
export const load: PageLoad = async ({ fetch }) => {
	const [doc, error] = await getDefaultLegalDocument('terms', fetch);

	return {
		content: doc?.content ?? '',
		updatedAt: doc?.updatedAt ?? null,
		loadError: error ? { message: error.message } : null
	};
};
