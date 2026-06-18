<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { getTenantUser } from '$lib/services/admin/users';
	import UserForm from '../../user-form.svelte';
	import { userFullName } from '../../display.js';

	const tenantId = $derived(page.params.tenantId!);
	const userId = $derived(page.params.userId!);
	const listHref = $derived(`/dashboard/${tenantId}/users`);

	const userQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId],
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await getTenantUser(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));

	const user = $derived(userQuery.data);
</script>

<svelte:head>
	<title>{user ? `Edit ${userFullName(user)}` : 'Edit user'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Users', path: listHref },
			{ name: user ? userFullName(user) : '…', path: `${listHref}/${userId}/edit-user` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to users', href: listHref }}
	title="Edit user"
	description="Update this user's identity. Email and password are managed through their own flows."
>
	{#if userQuery.isPending}
		<div class="text-muted-foreground mt-4 flex items-center gap-2">
			<LoadingSpinner /> Loading user…
		</div>
	{:else if userQuery.isError}
		<ErrorAlert title="Couldn't load this user" message={userQuery.error.message} />
	{:else if user}
		{#key user.user_id}
			<UserForm mode="edit" {tenantId} {user} />
		{/key}
	{/if}
</MainContent>
