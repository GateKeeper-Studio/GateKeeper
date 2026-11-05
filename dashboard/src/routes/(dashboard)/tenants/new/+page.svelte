<script lang="ts">
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import CreateTenantForm from './create-tenant-form.svelte';
	import { m } from '$lib/paraglide/messages.js';
</script>

<svelte:head>
	<title>{m.create_tenant_page_title()} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [{ name: m.create_tenant_breadcrumb(), path: '/tenants/new' }]
	}}
/>

<MainContent
	title={m.create_tenant_page_title()}
	description={m.create_tenant_page_description()}
	goBack={{ message: m.create_tenant_go_back(), href: '/' }}
>
	<CreateTenantForm />
</MainContent>
