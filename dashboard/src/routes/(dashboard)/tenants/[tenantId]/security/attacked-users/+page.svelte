<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listAttackedUsers } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	const usersQuery = createQuery(() => ({
		queryKey: ['attacked-users', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listAttackedUsers(tenantId, { limit: 100 });
			if (err) throw err;
			return data.items;
		}
	}));

	const fmt = (v: string | null) => (v ? new Date(v).toLocaleString() : '—');
</script>

<svelte:head><title>{m.security_attacked_users_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` },
			{ name: m.nav_attacked_users(), path: `/tenants/${tenantId}/security/attacked-users` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_attacked_users()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.security_attacked_users_description()}
		</p>
	</div>

	<div class="mt-6">
		{#if usersQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if usersQuery.isError}
			<ErrorAlert title={m.security_attacked_users_error_title()} message={usersQuery.error.message} />
		{:else}
			<div class="overflow-x-auto rounded-xl border">
				<table class="w-full text-left text-sm">
					<thead class="bg-muted/50 text-muted-foreground">
						<tr>
							<th class="px-4 py-2 font-medium">{m.security_label_user()}</th>
							<th class="px-4 py-2 font-medium">{m.security_attacked_users_col_failed_attempts()}</th>
							<th class="px-4 py-2 font-medium">{m.security_attacked_users_col_distinct_ips()}</th>
							<th class="px-4 py-2 font-medium">{m.security_attacked_users_col_last_failed()}</th>
							<th class="px-4 py-2 font-medium">{m.security_attacked_users_col_last_success()}</th>
						</tr>
					</thead>
					<tbody>
						{#each usersQuery.data as u (u.user_id)}
							<tr class="border-t">
								<td class="px-4 py-2 font-mono text-xs">{u.username ?? u.user_id.slice(0, 8)}</td>
								<td class="px-4 py-2">{u.failed_attempts}</td>
								<td class="px-4 py-2">{u.distinct_ips}</td>
								<td class="px-4 py-2 whitespace-nowrap">{fmt(u.last_failed_at)}</td>
								<td class="px-4 py-2 whitespace-nowrap">{fmt(u.last_success_at)}</td>
							</tr>
						{/each}
						{#if usersQuery.data.length === 0}
							<tr><td colspan="5" class="text-muted-foreground px-4 py-8 text-center">{m.security_attacked_users_empty()}</td></tr>
						{/if}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</MainContent>
