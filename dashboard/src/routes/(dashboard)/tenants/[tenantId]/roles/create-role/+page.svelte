<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import RoleForm from '../role-form.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/tenants/${tenantId}/roles`);
</script>

<svelte:head><title>{m.roles_create_role_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.roles_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.roles_page_title(), path: listHref },
			{ name: m.roles_create_role_title(), path: `${listHref}/create-role` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.roles_back_to_roles(), href: listHref }}
	title={m.roles_create_role_title()}
	description={m.roles_create_role_description()}
>
	<RoleForm mode="create" {tenantId} />
</MainContent>
