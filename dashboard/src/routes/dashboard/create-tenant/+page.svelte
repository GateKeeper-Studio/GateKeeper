<script lang="ts">
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import CreateTenantForm from './create-tenant-form.svelte';
</script>

<svelte:head>
	<title>Create Tenant - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: 'Create Tenant', path: '/dashboard/create-tenant' }
		]
	}}
/>

<MainContent
	title="Add Tenant"
	description="Here you can add a new tenant. Fill in the form below and click the &quot;Add Tenant&quot; button to
		create a new tenant."
	goBack={{ message: 'Go back to tenants list', href: '/dashboard' }}
>
	<CreateTenantForm />
</MainContent>
