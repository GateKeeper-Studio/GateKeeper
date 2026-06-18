<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import GroupForm from '../group-form.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/dashboard/${tenantId}/groups`);
</script>

<svelte:head><title>Create group - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Groups', path: listHref },
			{ name: 'Create group', path: `${listHref}/create-group` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to groups', href: listHref }}
	title="Create group"
	description="Define a group to organize users by role, team, or any logical category."
>
	<GroupForm mode="create" {tenantId} />
</MainContent>
