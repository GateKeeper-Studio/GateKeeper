import type { PageLoad } from './$types';
import { listEmailProviders } from '$lib/services/admin/email-providers';
import { listOAuthClients } from '$lib/services/admin/oauth-clients';

// Loads the tenant's SMTP providers and OAuth clients for the admin screen.
// Errors are returned (not thrown) so the page can render an alert.
export const load: PageLoad = async ({ params, fetch }) => {
	const [providers, providersError] = await listEmailProviders(params.tenantId, fetch);
	const [clients, clientsError] = await listOAuthClients(params.tenantId, fetch);

	const error = providersError ?? clientsError;

	return {
		tenantId: params.tenantId,
		providers: providers?.items ?? [],
		clients: clients?.items ?? [],
		loadError: error ? { message: error.message } : null
	};
};
