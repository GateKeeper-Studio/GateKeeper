<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getOAuthClient } from '$lib/services/admin/oauth-clients';
	import OAuthClientForm from '../../oauth-client-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const clientId = $derived(page.params.clientId!);
	const listHref = $derived(`/dashboard/${tenantId}/oauth-clients`);
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
	<title>{client ? `Edit ${client.name}` : 'Edit OAuth Client'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'OAuth Clients', path: listHref },
			{ name: client?.name ?? '…', path: detailHref },
			{ name: 'Edit', path: `${detailHref}/edit` }
		]
	}}
/>

<MainContent
	title="Edit OAuth Client"
	description="Update this client's redirect URIs, legal links, scopes, grant types, and policies. The client ID, secret, and type can't be changed here."
	goBack={{ message: 'Back to client', href: detailHref }}
>
	{#if clientQuery.isPending}
		<div class="mt-4 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> Loading client…
		</div>
	{:else if clientQuery.isError}
		<div class="mt-4">
			<ErrorAlert title="Couldn't load this client" message={clientQuery.error.message} />
		</div>
	{:else if client}
		{#key client.id}
			<OAuthClientForm mode="edit" {tenantId} {client} />
		{/key}
	{/if}
</MainContent>
