<script lang="ts">
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listLoginEvents } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	let page = $state(1);
	const pageSize = 25;
	let riskLevel = $state('');
	let decision = $state('');
	let successFilter = $state('');
	let ipAddress = $state('');

	// Reset to page 1 whenever a filter changes.
	function onFilterChange() {
		page = 1;
	}

	const eventsQuery = createQuery(() => ({
		queryKey: ['login-events', tenantId, page, riskLevel, decision, successFilter, ipAddress],
		enabled: !!tenantId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listLoginEvents(tenantId, {
				page,
				pageSize,
				riskLevel: riskLevel || undefined,
				decision: decision || undefined,
				success: successFilter === '' ? undefined : successFilter === 'true',
				ipAddress: ipAddress || undefined
			});
			if (err) throw err;
			return data;
		}
	}));

	const totalPages = $derived(Math.max(1, Math.ceil((eventsQuery.data?.total ?? 0) / pageSize)));

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

<svelte:head><title>{m.security_login_events_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` },
			{ name: m.nav_login_events(), path: `/tenants/${tenantId}/security/login-events` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_login_events()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.security_login_events_description()}
		</p>
	</div>

	<div class="mt-4 flex flex-wrap gap-3">
		<input
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			placeholder={m.security_login_events_filter_ip_placeholder()}
			bind:value={ipAddress}
			oninput={onFilterChange}
		/>
		<select
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			bind:value={riskLevel}
			onchange={onFilterChange}
		>
			<option value="">{m.security_login_events_filter_all_risk_levels()}</option>
			<option value="LOW">{m.security_login_events_risk_low()}</option>
			<option value="MEDIUM">{m.security_login_events_risk_medium()}</option>
			<option value="HIGH">{m.security_login_events_risk_high()}</option>
			<option value="CRITICAL">{m.security_login_events_risk_critical()}</option>
		</select>
		<select
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			bind:value={decision}
			onchange={onFilterChange}
		>
			<option value="">{m.security_login_events_filter_all_decisions()}</option>
			<option value="ALLOW">{m.security_login_events_decision_allow()}</option>
			<option value="ALLOW_WITH_MFA">{m.security_login_events_decision_allow_mfa()}</option>
			<option value="BLOCK_TEMPORARILY">{m.security_login_events_decision_block_temporarily()}</option>
			<option value="REVIEW">{m.security_login_events_decision_review()}</option>
		</select>
		<select
			class="border-input bg-background rounded-md border px-3 py-2 text-sm"
			bind:value={successFilter}
			onchange={onFilterChange}
		>
			<option value="">{m.security_login_events_filter_all_outcomes()}</option>
			<option value="true">{m.security_label_success()}</option>
			<option value="false">{m.security_login_events_outcome_failure()}</option>
		</select>
	</div>

	<div class="mt-6">
		{#if eventsQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if eventsQuery.isError}
			<ErrorAlert title={m.security_login_events_error_title()} message={eventsQuery.error.message} />
		{:else}
			<div class="overflow-x-auto rounded-xl border">
				<table class="w-full text-left text-sm">
					<thead class="bg-muted/50 text-muted-foreground">
						<tr>
							<th class="px-4 py-2 font-medium">{m.security_label_when()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_user()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_ip()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_country()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_success()}</th>
							<th class="px-4 py-2 font-medium">{m.security_login_events_col_score()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_risk()}</th>
							<th class="px-4 py-2 font-medium">{m.security_login_events_col_decision()}</th>
						</tr>
					</thead>
					<tbody>
						{#each eventsQuery.data.items as e (e.id)}
							<tr
								class="hover:bg-muted/40 cursor-pointer border-t"
								onclick={() =>
									(window.location.href = `/tenants/${tenantId}/security/login-events/${e.id}`)}
							>
								<td class="px-4 py-2 whitespace-nowrap">{new Date(e.created_at).toLocaleString()}</td>
								<td class="px-4 py-2 font-mono text-xs">{e.username_normalized ?? '—'}</td>
								<td class="px-4 py-2 font-mono text-xs">{e.ip_address ?? '—'}</td>
								<td class="px-4 py-2">{e.country_code ?? '—'}</td>
								<td class="px-4 py-2">
									<span class:text-green-600={e.success} class:text-red-600={!e.success}>
										{e.success ? m.security_yes() : m.security_no()}
									</span>
								</td>
								<td class="px-4 py-2">{e.risk_score}</td>
								<td class="px-4 py-2">
									<span class="rounded px-2 py-0.5 text-xs font-medium {riskClass(e.risk_level)}">
										{e.risk_level}
									</span>
								</td>
								<td class="px-4 py-2 font-mono text-xs">{e.decision}</td>
							</tr>
						{/each}
						{#if eventsQuery.data.items.length === 0}
							<tr><td colspan="8" class="text-muted-foreground px-4 py-8 text-center">{m.security_login_events_empty()}</td></tr>
						{/if}
					</tbody>
				</table>
			</div>

			<div class="mt-4 flex items-center justify-between text-sm">
				<span class="text-muted-foreground">
					{m.security_login_events_pagination_summary({ total: eventsQuery.data.total, page, totalPages })}
				</span>
				<div class="flex gap-2">
					<button
						class="border-input rounded-md border px-3 py-1 disabled:opacity-50"
						disabled={page <= 1}
						onclick={() => (page = Math.max(1, page - 1))}
					>
						{m.security_login_events_pagination_previous()}
					</button>
					<button
						class="border-input rounded-md border px-3 py-1 disabled:opacity-50"
						disabled={page >= totalPages}
						onclick={() => (page = Math.min(totalPages, page + 1))}
					>
						{m.security_login_events_pagination_next()}
					</button>
				</div>
			</div>
		{/if}
	</div>
</MainContent>
