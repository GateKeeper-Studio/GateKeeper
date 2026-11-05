<script lang="ts">
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listAuditEvents } from '$lib/services/admin/audit';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	let page = $state(1);
	const pageSize = 25;
	let outcome = $state('');
	let action = $state('');

	// Reset to page 1 whenever a filter changes.
	function onFilterChange() {
		page = 1;
	}

	const auditQuery = createQuery(() => ({
		queryKey: ['tenant-audit', tenantId, page, outcome, action],
		enabled: !!tenantId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listAuditEvents(tenantId, {
				page,
				pageSize,
				outcome: outcome || undefined,
				action: action || undefined
			});
			if (err) throw err;
			return data;
		}
	}));

	const totalPages = $derived(Math.max(1, Math.ceil((auditQuery.data?.total ?? 0) / pageSize)));
</script>

<svelte:head><title>{m.nav_audit_log()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_audit_log(), path: `/tenants/${tenantId}/audit` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_audit_log()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.tenant_audit_description()}
		</p>
	</div>

	<div class="mt-4 flex flex-wrap gap-3">
		<input
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			placeholder={m.tenant_audit_filter_action_placeholder()}
			bind:value={action}
			oninput={onFilterChange}
		/>
		<select
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			bind:value={outcome}
			onchange={onFilterChange}
		>
			<option value="">{m.tenant_audit_all_outcomes()}</option>
			<option value="success">{m.tenant_audit_outcome_success()}</option>
			<option value="failure">{m.tenant_audit_outcome_failure()}</option>
		</select>
	</div>

	<div class="mt-6">
		{#if auditQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if auditQuery.isError}
			<ErrorAlert title={m.tenant_audit_load_error_title()} message={auditQuery.error.message} />
		{:else}
			<div class="overflow-x-auto rounded-xl border">
				<table class="w-full text-left text-sm">
					<thead class="bg-muted/50 text-muted-foreground">
						<tr>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_when()}</th>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_action()}</th>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_outcome()}</th>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_actor()}</th>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_resource()}</th>
							<th class="px-4 py-2 font-medium">{m.tenant_audit_table_ip()}</th>
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
							<tr><td colspan="6" class="text-muted-foreground px-4 py-8 text-center">{m.tenant_audit_no_events()}</td></tr>
						{/if}
					</tbody>
				</table>
			</div>

			<div class="mt-4 flex items-center justify-between text-sm">
				<span class="text-muted-foreground">
					{m.tenant_audit_pagination_summary({ total: auditQuery.data.total, page, totalPages })}
				</span>
				<div class="flex gap-2">
					<button
						class="border-input rounded-md border px-3 py-1 disabled:opacity-50"
						disabled={page <= 1}
						onclick={() => (page = Math.max(1, page - 1))}
					>
						{m.tenant_audit_pagination_previous()}
					</button>
					<button
						class="border-input rounded-md border px-3 py-1 disabled:opacity-50"
						disabled={page >= totalPages}
						onclick={() => (page = Math.min(totalPages, page + 1))}
					>
						{m.tenant_audit_pagination_next()}
					</button>
				</div>
			</div>
		{/if}
	</div>
</MainContent>
