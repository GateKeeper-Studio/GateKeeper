<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import UserForm from '../user-form.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/tenants/${tenantId}/users`);
</script>

<svelte:head><title>{m.users_add_user_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.users_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.users_page_title(), path: listHref },
			{ name: m.users_add_user_title(), path: `${listHref}/create-user` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.users_back_to_list(), href: listHref }}
	title={m.users_add_user_title()}
	description={m.users_create_page_description()}
>
	<UserForm mode="create" {tenantId} />
</MainContent>
