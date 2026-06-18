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

<svelte:head><title>Groups - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Groups', path: `/dashboard/${tenantId}/groups` }
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
	goBack={{ message: "Back to tenant detail", href: `/dashboard/${tenantId}` }}
>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">Groups</h1>

			<p class="text-muted-foreground max-w-2xl text-sm">
				Collections of users that share access policies. Add members to a group to manage their
				permissions together.
			</p>
		</div>

		<Button href={`/dashboard/${tenantId}/groups/create-group`}>
			<PlusIcon />
			Create group
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
			<ErrorAlert title="Couldn't load groups" message={groupsQuery.error.message} />
		{:else}
			<GroupsTable {tenantId} />
		{/if}
	</div>
</MainContent>
