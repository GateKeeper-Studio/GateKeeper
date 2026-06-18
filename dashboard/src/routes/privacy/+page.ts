import type { PageLoad } from './$types';
import { getDefaultLegalDocument } from '$lib/services/auth/get-legal-document';

// The bare /privacy route has no client/tenant context, so it shows the platform
// default Privacy Policy.
export const load: PageLoad = async ({ fetch }) => {
	const [doc, error] = await getDefaultLegalDocument('privacy', fetch);

	return {
		content: doc?.content ?? '',
		updatedAt: doc?.updatedAt ?? null,
		loadError: error ? { message: error.message } : null
	};
};
