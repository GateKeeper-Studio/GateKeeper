<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import PlusIcon from '@lucide/svelte/icons/plus';
	import UsersIcon from '@lucide/svelte/icons/users';
	import UserPlusIcon from '@lucide/svelte/icons/user-plus';
	import ClockIcon from '@lucide/svelte/icons/clock';

	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listTenantUsers } from '$lib/services/admin/users';
	import UsersTable from './users-table.svelte';

	const tenantId = $derived(selectedTenant.tenantId!);

	const usersQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantUsers(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	const users = $derived(usersQuery.data ?? []);

	// "Joined in the last 30 days" — a light at-a-glance signal of recent growth.
	const recentCount = $derived.by(() => {
		const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000;
		return users.filter((u) => {
			const t = new Date(u.created_at).getTime();
			return !Number.isNaN(t) && t >= cutoff;
		}).length;
	});
</script>

<svelte:head><title>Users - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Users', path: `/dashboard/${tenantId}/users` }
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
			<h1 class="text-3xl font-bold tracking-tight">Users</h1>

			<p class="text-muted-foreground max-w-2xl text-sm">
				People who belong to this tenant. They can sign in to its applications and be assigned
				access across them.
			</p>
		</div>

		<Button href={`/dashboard/${tenantId}/users/create-user`}>
			<PlusIcon />
			Add user
		</Button>
	</div>

	<div class="mt-6">
		{#if usersQuery.isPending}
			<div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
				{#each Array(3) as _, i (i)}
					<Skeleton class="h-[72px] w-full rounded-xl" />
				{/each}
			</div>

			<Skeleton class="mt-4 h-64 w-full rounded-xl" />
		{:else if usersQuery.isError}
			<ErrorAlert title="Couldn't load users" message={usersQuery.error.message} />
		{:else}
			<div class="flex gap-4">
				{@render statTile(
					'Total users',
					String(users.length),
					UsersIcon,
					'bg-sky-500/10 text-sky-600 dark:text-sky-400'
				)}
				{@render statTile(
					'New this month',
					String(recentCount),
					UserPlusIcon,
					'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
				)}
				{@render statTile(
					'Tenant',
					selectedTenant.current?.name ?? '—',
					ClockIcon,
					'bg-violet-500/10 text-violet-600 dark:text-violet-400'
				)}
			</div>

			<div class="mt-4">
				<UsersTable {tenantId} />
			</div>
		{/if}
	</div>
</MainContent>
