import type { PageLoad } from './$types';
import { listEmailProviders } from '$lib/services/admin/email-providers';
import { listEmailTemplates } from '$lib/services/admin/email-templates';
import { listOAuthClients } from '$lib/services/admin/oauth-clients';

const VALID_TABS = new Set(['providers', 'templates']);

export const load: PageLoad = async ({ params, fetch, url }) => {
	const [providers, providersError] = await listEmailProviders(params.tenantId, fetch);
	const [clients, clientsError] = await listOAuthClients(params.tenantId, fetch);
	const [templates, templatesError] = await listEmailTemplates(params.tenantId, fetch);

	const activeTab = url.searchParams.get('tab') ?? 'providers';

	return {
		tenantId: params.tenantId,
		activeTab: VALID_TABS.has(activeTab) ? activeTab : 'providers',
		providers: providers?.items ?? [],
		clients: clients?.items ?? [],
		templates: templates?.items ?? [],
		providersLoadError:
			providersError || clientsError
				? { message: (providersError ?? clientsError)!.message }
				: null,
		templatesLoadError: templatesError ? { message: templatesError.message } : null
	};
};
