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
	import { m } from '$lib/paraglide/messages.js';

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

<svelte:head><title>{m.roles_page_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.roles_tenant_fallback(), path: `/tenants/${tenantId}` },
			{ name: m.roles_page_title(), path: `/tenants/${tenantId}/roles` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.roles_back_to_tenant_detail(), href: `/tenants/${tenantId}` }}
>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">{m.roles_page_title()}</h1>

			<p class="text-muted-foreground max-w-2xl text-sm">
				{m.roles_page_description()}
			</p>
		</div>

		<Button href={`/tenants/${tenantId}/roles/create-role`}>
			<PlusIcon />
			{m.roles_create_role_title()}
		</Button>
	</div>

	<div class="mt-6">
		{#if rolesQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if rolesQuery.isError}
			<ErrorAlert title={m.roles_load_error_title()} message={rolesQuery.error.message} />
		{:else}
			<RolesTable {tenantId} />
		{/if}
	</div>
</MainContent>
