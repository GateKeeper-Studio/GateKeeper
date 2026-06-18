<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { getTenantGroup } from '$lib/services/admin/groups';
	import GroupForm from '../../group-form.svelte';

	const tenantId = $derived(page.params.tenantId!);
	const groupId = $derived(page.params.groupId!);
	const listHref = $derived(`/dashboard/${tenantId}/groups`);

	const groupQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId, groupId],
		enabled: !!tenantId && !!groupId,
		queryFn: async () => {
			const [data, err] = await getTenantGroup(tenantId, groupId);
			if (err) throw err;
			return data;
		}
	}));

	const group = $derived(groupQuery.data);
</script>

<svelte:head>
	<title>{group ? `Edit ${group.name}` : 'Edit group'} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Groups', path: listHref },
			{ name: group?.name ?? '…', path: `${listHref}/${groupId}` },
			{ name: 'Edit', path: `${listHref}/${groupId}/edit-group` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to groups', href: listHref }}
	title="Edit group"
	description="Update this group's name and description."
>
	{#if groupQuery.isPending}
		<div class="text-muted-foreground mt-4 flex items-center gap-2">
			<LoadingSpinner /> Loading group…
		</div>
	{:else if groupQuery.isError}
		<ErrorAlert title="Couldn't load this group" message={groupQuery.error.message} />
	{:else if group}
		{#key group.group_id}
			<GroupForm mode="edit" {tenantId} {group} />
		{/key}
	{/if}
</MainContent>
