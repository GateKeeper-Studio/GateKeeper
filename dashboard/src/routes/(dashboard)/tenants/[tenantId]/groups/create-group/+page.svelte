<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import GroupForm from '../group-form.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/tenants/${tenantId}/groups`);
</script>

<svelte:head><title>{m.groups_create_group_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{
				name: selectedTenant.current?.name ?? m.groups_breadcrumb_tenant_fallback(),
				path: `/tenants/${tenantId}`
			},
			{ name: m.groups_page_title(), path: listHref },
			{ name: m.groups_create_group_title(), path: `${listHref}/create-group` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.groups_back_to_groups(), href: listHref }}
	title={m.groups_create_group_title()}
	description={m.groups_create_group_description()}
>
	<GroupForm mode="create" {tenantId} />
</MainContent>
