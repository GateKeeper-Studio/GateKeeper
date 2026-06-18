<script lang="ts">
	import { page } from '$app/state';

	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import OAuthClientForm from '../oauth-client-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const listHref = $derived(`/dashboard/${tenantId}/oauth-clients`);
</script>

<svelte:head><title>Create OAuth Client - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'OAuth Clients', path: listHref },
			{ name: 'Create Client', path: `${listHref}/create-oauth-client` }
		]
	}}
/>

<MainContent
	title="Create OAuth Client"
	description="Register a new application with the identity provider. Set its redirect URIs, scopes, grant types, and PKCE / consent policy below."
	goBack={{ message: 'Back to OAuth clients', href: listHref }}
>
	<OAuthClientForm mode="create" {tenantId} />
</MainContent>
