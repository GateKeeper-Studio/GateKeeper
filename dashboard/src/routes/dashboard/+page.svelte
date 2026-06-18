<script lang="ts">
	import MainContent from '$lib/components/main-content.svelte';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
</script>

<svelte:head>
	<title>Dashboard - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: 'Dashboard', path: '/dashboard' },
		]
	}}
/>

<MainContent
	title={selectedTenant.current ? selectedTenant.current.name : 'Dashboard'}
	description={selectedTenant.current?.description ? selectedTenant.current.description : 'Select a tenant from the sidebar to get started.'}
>
	<div></div>
</MainContent>
