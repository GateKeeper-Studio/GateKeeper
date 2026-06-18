<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { getTenantRole } from '$lib/services/admin/roles';
	import RoleForm from '../../role-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const roleId = $derived(page.params.roleId!);
	const listHref = $derived(`/dashboard/${tenantId}/roles`);

	const roleQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId, roleId],
		enabled: !!tenantId && !!roleId,
		queryFn: async () => {
			const [data, err] = await getTenantRole(tenantId, roleId);
			if (err) throw err;
			return data;
		}
	}));

	const role = $derived(roleQuery.data);
</script>

<svelte:head>
	<title>{role ? `Edit ${role.name}` : 'Edit role'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Roles', path: listHref },
			{ name: role?.name ?? '…', path: `${listHref}/${roleId}` },
			{ name: 'Edit', path: `${listHref}/${roleId}/edit-role` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to roles', href: listHref }}
	title="Edit role"
	description="Update this role's name and description."
>
	{#if roleQuery.isPending}
		<div class="text-muted-foreground mt-4 flex items-center gap-2">
			<LoadingSpinner /> Loading role…
		</div>
	{:else if roleQuery.isError}
		<ErrorAlert title="Couldn't load this role" message={roleQuery.error.message} />
	{:else if role}
		{#key role.role_id}
			<RoleForm mode="edit" {tenantId} {role} />
		{/key}
	{/if}
</MainContent>
