<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import PlusIcon from '@lucide/svelte/icons/plus';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import CalendarPlusIcon from '@lucide/svelte/icons/calendar-plus';

	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listTenantGroups } from '$lib/services/admin/groups';
	import { m } from '$lib/paraglide/messages.js';
	import GroupsTable from './groups-table.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);

	const groupsQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantGroups(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	const groups = $derived(groupsQuery.data ?? []);

	const recentCount = $derived.by(() => {
		const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000;
		return groups.filter((g) => {
			const t = new Date(g.created_at).getTime();
			return !Number.isNaN(t) && t >= cutoff;
		}).length;
	});
</script>

<svelte:head><title>{m.groups_page_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{
				name: selectedTenant.current?.name ?? m.groups_breadcrumb_tenant_fallback(),
				path: `/tenants/${tenantId}`
			},
			{ name: m.groups_page_title(), path: `/tenants/${tenantId}/groups` }
		]
	}}
/>

{#snippet statTile(label: string, value: string, Icon: any, chip: string)}
	<div class="bg-card flex items-center gap-3 rounded-xl border p-2">
		<span class={cn('flex size-10 shrink-0 items-center justify-center rounded-lg', chip)}>
			<Icon class="size-5" />
		</span>

		<div class="min-w-0">
			<p class="text-muted-foreground text-xs font-medium">{label}</p>
			<p class="truncate text-xl font-semibold">{value}</p>
		</div>
	</div>
{/snippet}

<MainContent
	goBack={{ message: m.groups_back_to_tenant(), href: `/tenants/${tenantId}` }}
>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">{m.groups_page_title()}</h1>

			<p class="text-muted-foreground max-w-2xl text-sm">
				{m.groups_page_description()}
			</p>
		</div>

		<Button href={`/tenants/${tenantId}/groups/create-group`}>
			<PlusIcon />
			{m.groups_create_group_title()}
		</Button>
	</div>

	<div class="mt-6">
		{#if groupsQuery.isPending}
			<div class="grid gap-3 sm:grid-cols-2">
				{#each Array(2) as _, i (i)}
					<Skeleton class="h-[72px] w-full rounded-xl" />
				{/each}
			</div>

			<Skeleton class="mt-4 h-64 w-full rounded-xl" />
		{:else if groupsQuery.isError}
			<ErrorAlert title={m.groups_load_groups_error_title()} message={groupsQuery.error.message} />
		{:else}
			<GroupsTable {tenantId} />
		{/if}
	</div>
</MainContent>
