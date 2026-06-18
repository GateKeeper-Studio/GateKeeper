<script lang="ts">
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import UserForm from '../user-form.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);
	const listHref = $derived(`/dashboard/${tenantId}/users`);
</script>

<svelte:head><title>Add user - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Users', path: listHref },
			{ name: 'Add user', path: `${listHref}/create-user` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to users', href: listHref }}
	title="Add user"
	description="Create a new user in this tenant. They'll sign in with the temporary password and set their own on first login."
>
	<UserForm mode="create" {tenantId} />
</MainContent>
