<script lang="ts">
	import { page } from '$app/state';

	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import OAuthClientForm from '../oauth-client-form.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const listHref = $derived(`/tenants/${tenantId}/oauth-clients`);
</script>

<svelte:head><title>{m.oauth_clients_create_page_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_oauth_clients(), path: listHref },
			{ name: m.oauth_clients_create_breadcrumb(), path: `${listHref}/create-oauth-client` }
		]
	}}
/>

<MainContent
	title={m.oauth_clients_create_page_title()}
	description={m.oauth_clients_create_page_description()}
	goBack={{ message: m.oauth_clients_back_to_clients(), href: listHref }}
>
	<OAuthClientForm mode="create" {tenantId} />
</MainContent>
