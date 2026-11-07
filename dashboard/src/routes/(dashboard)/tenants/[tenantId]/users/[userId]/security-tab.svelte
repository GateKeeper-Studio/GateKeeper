<script lang="ts">
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';

	import { listLoginEvents, type LoginEventView } from '$lib/services/admin/security';
	import SessionsSection from './sessions-section.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { tenantId, userId }: { tenantId: string; userId: string } = $props();

	let page = $state(1);
	const pageSize = 10;

	const eventsQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId, 'login-events', page],
		enabled: !!tenantId && !!userId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listLoginEvents(tenantId, { userId, page, pageSize });
			if (err) throw err;
			return data;
		}
	}));

	const totalPages = $derived(Math.max(1, Math.ceil((eventsQuery.data?.total ?? 0) / pageSize)));

	function location(e: LoginEventView): string {
		const parts = [e.city, e.region, e.country_code].filter(Boolean);
		return parts.length ? parts.join(', ') : '—';
	}

	function riskClass(level: string): string {
		switch (level) {
			case 'CRITICAL':
				return 'bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300';
			case 'HIGH':
				return 'bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300';
			case 'MEDIUM':
				return 'bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300';
			default:
				return 'bg-muted text-muted-foreground';
		}
	}
</script>

<SessionsSection {tenantId} {userId} />

<div class="mt-8 space-y-1">
	<h2 class="text-lg font-semibold">{m.users_security_activity_title()}</h2>
	<p class="max-w-2xl text-sm text-muted-foreground">
		{m.users_security_activity_description()}
	</p>
</div>

<div class="mt-4">
	{#if eventsQuery.isPending}
		<Skeleton class="h-64 w-full rounded-xl" />
	{:else if eventsQuery.isError}
		<ErrorAlert title={m.users_security_load_error_title()} message={eventsQuery.error.message} />
	{:else}
		<div class="overflow-x-auto rounded-xl border">
			<table class="w-full text-left text-sm">
				<thead class="bg-muted/50 text-muted-foreground">
					<tr>
						<th class="px-4 py-2 font-medium">{m.users_security_col_when()}</th>
						<th class="px-4 py-2 font-medium">{m.users_security_col_location()}</th>
						<th class="px-4 py-2 font-medium">{m.users_security_col_ip()}</th>
						<th class="px-4 py-2 font-medium">{m.users_security_col_success()}</th>
						<th class="px-4 py-2 font-medium">{m.users_security_col_risk()}</th>
						<th class="px-4 py-2 font-medium">{m.users_security_col_decision()}</th>
					</tr>
				</thead>
				<tbody>
					{#each eventsQuery.data.items as e (e.id)}
						<tr
							class="cursor-pointer border-t hover:bg-muted/40"
							onclick={() =>
								(window.location.href = `/tenants/${tenantId}/security/login-events/${e.id}`)}
						>
							<td class="px-4 py-2 whitespace-nowrap">{new Date(e.created_at).toLocaleString()}</td>
							<td class="px-4 py-2">{location(e)}</td>
							<td class="px-4 py-2 font-mono text-xs">{e.ip_address ?? '—'}</td>
							<td class="px-4 py-2">
								<span class:text-green-600={e.success} class:text-red-600={!e.success}>
									{e.success ? m.users_security_success_yes() : m.users_security_success_no()}
								</span>
							</td>
							<td class="px-4 py-2">
								<span class="rounded px-2 py-0.5 text-xs font-medium {riskClass(e.risk_level)}">
									{e.risk_level}
								</span>
							</td>
							<td class="px-4 py-2 font-mono text-xs">{e.decision}</td>
						</tr>
					{/each}
					{#if eventsQuery.data.items.length === 0}
						<tr>
							<td colspan="6" class="px-4 py-8 text-center text-muted-foreground">
								{m.users_security_no_activity()}
							</td>
						</tr>
					{/if}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="mt-4 flex items-center justify-between text-sm">
				<span class="text-muted-foreground">
					{m.users_security_pagination_summary({
						total: eventsQuery.data.total,
						page,
						totalPages
					})}
				</span>
				<div class="flex gap-2">
					<button
						class="rounded-md border border-input px-3 py-1 disabled:opacity-50"
						disabled={page <= 1}
						onclick={() => (page = Math.max(1, page - 1))}
					>
						{m.users_action_previous()}
					</button>
					<button
						class="rounded-md border border-input px-3 py-1 disabled:opacity-50"
						disabled={page >= totalPages}
						onclick={() => (page = Math.min(totalPages, page + 1))}
					>
						{m.users_action_next()}
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>
