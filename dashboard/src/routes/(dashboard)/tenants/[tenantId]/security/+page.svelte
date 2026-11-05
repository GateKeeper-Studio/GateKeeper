<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getSecurityOverview } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	const overviewQuery = createQuery(() => ({
		queryKey: ['security-overview', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await getSecurityOverview(tenantId);
			if (err) throw err;
			return data;
		}
	}));

	const maxCountry = $derived(
		Math.max(1, ...(overviewQuery.data?.logins_by_country ?? []).map((c) => c.count))
	);
</script>

<svelte:head><title>{m.security_overview_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.security_overview_heading()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.security_overview_description()}
		</p>
	</div>

	{#if overviewQuery.isPending}
		<Skeleton class="mt-6 h-64 w-full rounded-xl" />
	{:else if overviewQuery.isError}
		<ErrorAlert title={m.security_overview_error_title()} message={overviewQuery.error.message} />
	{:else}
		{@const metrics = overviewQuery.data.metrics}
		<div class="mt-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
			{@render kpi(m.security_overview_kpi_total_attempts(), metrics.total_attempts, 'text-foreground')}
			{@render kpi(m.security_overview_kpi_successful(), metrics.successful_logins, 'text-green-600')}
			{@render kpi(m.security_label_failed(), metrics.failed_logins, 'text-amber-600')}
			{@render kpi(m.security_overview_kpi_blocked(), metrics.blocked_attempts, 'text-red-600')}
			{@render kpi(m.security_overview_kpi_high_risk(), metrics.high_risk_attempts, 'text-orange-600')}
			{@render kpi(m.security_overview_kpi_critical_risk(), metrics.critical_risk_attempts, 'text-red-700')}
			{@render kpi(m.security_overview_kpi_password_spraying(), metrics.password_spraying_events, 'text-red-600')}
		</div>

		<div class="mt-8 grid gap-6 lg:grid-cols-2">
			<div class="rounded-xl border">
				<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_overview_top_suspicious_ips()}</div>
				<div class="overflow-x-auto">
					<table class="w-full text-left text-sm">
						<thead class="bg-muted/50 text-muted-foreground">
							<tr>
								<th class="px-4 py-2 font-medium">{m.security_label_ip()}</th>
								<th class="px-4 py-2 font-medium">{m.security_label_country()}</th>
								<th class="px-4 py-2 font-medium">{m.security_label_failed()}</th>
								<th class="px-4 py-2 font-medium">{m.security_label_users()}</th>
								<th class="px-4 py-2 font-medium">{m.security_label_max_score()}</th>
							</tr>
						</thead>
						<tbody>
							{#each overviewQuery.data.top_suspicious_ips as ip (ip.ip_address)}
								<tr class="border-t">
									<td class="px-4 py-2 font-mono text-xs">{ip.ip_address}</td>
									<td class="px-4 py-2">{ip.country_code ?? '—'}</td>
									<td class="px-4 py-2">{ip.failed_attempts}</td>
									<td class="px-4 py-2">{ip.distinct_users}</td>
									<td class="px-4 py-2">{ip.max_risk_score}</td>
								</tr>
							{/each}
							{#if overviewQuery.data.top_suspicious_ips.length === 0}
								<tr><td colspan="5" class="text-muted-foreground px-4 py-8 text-center">{m.security_no_suspicious_ips()}</td></tr>
							{/if}
						</tbody>
					</table>
				</div>
			</div>

			<div class="rounded-xl border">
				<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_overview_logins_by_country()}</div>
				<div class="space-y-2 p-4">
					{#each overviewQuery.data.logins_by_country.slice(0, 10) as c (c.country_code ?? 'unknown')}
						<div class="flex items-center gap-3">
							<span class="w-10 shrink-0 font-mono text-xs">{c.country_code ?? '??'}</span>
							<div class="bg-muted h-3 flex-1 overflow-hidden rounded">
								<div class="bg-primary h-full" style="width: {(c.count / maxCountry) * 100}%"></div>
							</div>
							<span class="text-muted-foreground w-10 shrink-0 text-right text-xs">{c.count}</span>
						</div>
					{/each}
					{#if overviewQuery.data.logins_by_country.length === 0}
						<p class="text-muted-foreground py-6 text-center text-sm">{m.security_overview_no_data()}</p>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</MainContent>

{#snippet kpi(label: string, value: number, valueClass: string)}
	<div class="bg-card flex flex-col gap-1 rounded-xl border p-4">
		<p class="text-muted-foreground text-xs">{label}</p>
		<p class="text-2xl font-semibold {valueClass}">{value}</p>
	</div>
{/snippet}
