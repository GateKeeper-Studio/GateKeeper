<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listAuditEvents } from '$lib/services/admin/audit';

	const tenantId = $derived(selectedTenant.tenantId!);

	let outcome = $state('');
	let action = $state('');

	const auditQuery = createQuery(() => ({
		queryKey: ['tenant-audit', tenantId, outcome, action],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listAuditEvents(tenantId, {
				outcome: outcome || undefined,
				action: action || undefined,
				pageSize: 100
			});
			if (err) throw err;
			return data;
		}
	}));
</script>

<svelte:head><title>Audit Log - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Audit Log', path: `/dashboard/${tenantId}/audit` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">Audit Log</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			Append-only record of security and compliance events (LGPD/GDPR/ISO 27001/SOC 2). Filter by
			action or outcome to investigate activity.
		</p>
	</div>

	<div class="mt-4 flex flex-wrap gap-3">
		<input
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			placeholder="Filter by action (e.g. user.login.success)"
			bind:value={action}
		/>
		<select
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			bind:value={outcome}
		>
			<option value="">All outcomes</option>
			<option value="success">Success</option>
			<option value="failure">Failure</option>
		</select>
	</div>

	<div class="mt-6">
		{#if auditQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if auditQuery.isError}
			<ErrorAlert title="Couldn't load audit events" message={auditQuery.error.message} />
		{:else}
			<div class="overflow-x-auto rounded-xl border">
				<table class="w-full text-left text-sm">
					<thead class="bg-muted/50 text-muted-foreground">
						<tr>
							<th class="px-4 py-2 font-medium">When</th>
							<th class="px-4 py-2 font-medium">Action</th>
							<th class="px-4 py-2 font-medium">Outcome</th>
							<th class="px-4 py-2 font-medium">Actor</th>
							<th class="px-4 py-2 font-medium">Resource</th>
							<th class="px-4 py-2 font-medium">IP</th>
						</tr>
					</thead>
					<tbody>
						{#each auditQuery.data.items as event (event.id)}
							<tr class="border-t">
								<td class="px-4 py-2 whitespace-nowrap">{new Date(event.created_at).toLocaleString()}</td>
								<td class="px-4 py-2 font-mono text-xs">{event.action}</td>
								<td class="px-4 py-2">
									<span
										class:text-green-600={event.outcome === 'success'}
										class:text-red-600={event.outcome === 'failure'}
									>
										{event.outcome}
									</span>
								</td>
								<td class="px-4 py-2 font-mono text-xs">{event.actor_type}{event.actor_id ? `:${event.actor_id.slice(0, 8)}` : ''}</td>
								<td class="px-4 py-2 font-mono text-xs">{event.resource_type ?? '—'}</td>
								<td class="px-4 py-2 font-mono text-xs">{event.ip_address ?? '—'}</td>
							</tr>
						{/each}
						{#if auditQuery.data.items.length === 0}
							<tr><td colspan="6" class="text-muted-foreground px-4 py-8 text-center">No audit events.</td></tr>
						{/if}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</MainContent>
