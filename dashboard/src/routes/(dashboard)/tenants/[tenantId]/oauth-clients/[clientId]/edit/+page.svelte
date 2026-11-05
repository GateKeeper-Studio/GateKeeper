<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getOAuthClient } from '$lib/services/admin/oauth-clients';
	import { m } from '$lib/paraglide/messages.js';
	import OAuthClientForm from '../../oauth-client-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const clientId = $derived(page.params.clientId!);
	const listHref = $derived(`/tenants/${tenantId}/oauth-clients`);
	const detailHref = $derived(`${listHref}/${clientId}`);

	// Reuses the detail page's query key, so it's served from cache when arriving
	// from "Edit" and stays in sync after a successful save invalidates it.
	const clientQuery = createQuery(() => ({
		queryKey: ['oauth-clients', tenantId, clientId],
		enabled: !!tenantId && !!clientId,
		queryFn: async () => {
			const [data, err] = await getOAuthClient(tenantId, clientId);
			if (err) throw err;
			return data;
		}
	}));

	const client = $derived(clientQuery.data);
</script>

<svelte:head>
	<title
		>{client
			? m.oauth_clients_edit_page_title_with_name({ name: client.name })
			: m.oauth_clients_edit_page_title()} - GateKeeper</title
	>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/tenants/${tenantId}` },
			{ name: m.nav_oauth_clients(), path: listHref },
			{ name: client?.name ?? '…', path: detailHref },
			{ name: m.oauth_clients_action_edit(), path: `${detailHref}/edit` }
		]
	}}
/>

<MainContent
	title={m.oauth_clients_edit_page_title()}
	description={m.oauth_clients_edit_page_description()}
	goBack={{ message: m.oauth_clients_edit_page_back_to_client(), href: detailHref }}
>
	{#if clientQuery.isPending}
		<div class="mt-4 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> {m.oauth_clients_loading_client()}
		</div>
	{:else if clientQuery.isError}
		<div class="mt-4">
			<ErrorAlert title={m.oauth_clients_load_error_title()} message={clientQuery.error.message} />
		</div>
	{:else if client}
		{#key client.id}
			<OAuthClientForm mode="edit" {tenantId} {client} />
		{/key}
	{/if}
</MainContent>
