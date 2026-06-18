<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import RoleForm from '../role-form.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/dashboard/${tenantId}/roles`);
</script>

<svelte:head><title>Create role - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Roles', path: listHref },
			{ name: 'Create role', path: `${listHref}/create-role` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to roles', href: listHref }}
	title="Create role"
	description="Define a role to grant a consistent set of permissions to the users you assign it to."
>
	<RoleForm mode="create" {tenantId} />
</MainContent>
