<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import PlusIcon from '@lucide/svelte/icons/plus';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listTenantRoles } from '$lib/services/admin/roles';
	import RolesTable from './roles-table.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);

	const rolesQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantRoles(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));
</script>

<svelte:head><title>Roles - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Roles', path: `/dashboard/${tenantId}/roles` }
		]
	}}
/>

<MainContent 
	goBack={{ message: "Back to tenant detail", href: `/dashboard/${tenantId}` }}
>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">Roles</h1>

			<p class="text-muted-foreground max-w-2xl text-sm">
				Named sets of permissions you assign to users. Assign a role to grant its access to
				everyone who holds it.
			</p>
		</div>

		<Button href={`/dashboard/${tenantId}/roles/create-role`}>
			<PlusIcon />
			Create role
		</Button>
	</div>

	<div class="mt-6">
		{#if rolesQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if rolesQuery.isError}
			<ErrorAlert title="Couldn't load roles" message={rolesQuery.error.message} />
		{:else}
			<RolesTable {tenantId} />
		{/if}
	</div>
</MainContent>
