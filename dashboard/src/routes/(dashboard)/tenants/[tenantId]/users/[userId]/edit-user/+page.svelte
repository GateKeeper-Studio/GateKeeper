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
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const userId = $derived(page.params.userId!);
	const listHref = $derived(`/tenants/${tenantId}/users`);

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
			{ name: selectedTenant.current?.name ?? m.users_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.users_page_title(), path: listHref },
			{
				name: user ? userFullName(user) : m.users_breadcrumb_loading_placeholder(),
				path: `${listHref}/${userId}/edit-user`
			}
		]
	}}
/>

<MainContent
	goBack={{ message: m.users_back_to_list(), href: listHref }}
	title={m.users_action_edit_user()}
	description={m.users_edit_user_description()}
>
	{#if userQuery.isPending}
		<div class="text-muted-foreground mt-4 flex items-center gap-2">
			<LoadingSpinner /> {m.users_loading_user()}
		</div>
	{:else if userQuery.isError}
		<ErrorAlert title={m.users_load_user_error_title()} message={userQuery.error.message} />
	{:else if user}
		{#key user.user_id}
			<UserForm mode="edit" {tenantId} {user} />
		{/key}
	{/if}
</MainContent>
