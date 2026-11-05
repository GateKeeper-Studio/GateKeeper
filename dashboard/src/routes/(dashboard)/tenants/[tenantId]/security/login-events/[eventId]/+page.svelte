<script lang="ts">
	import { page } from '$app/state';
	import { createQuery } from '@tanstack/svelte-query';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { getLoginEvent, type LoginEventView } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(page.params.tenantId!);
	const eventId = $derived(page.params.eventId!);

	const detailQuery = createQuery(() => ({
		queryKey: ['login-event', tenantId, eventId],
		enabled: !!tenantId && !!eventId,
		queryFn: async () => {
			const [data, err] = await getLoginEvent(tenantId, eventId);
			if (err) throw err;
			return data;
		}
	}));

	function field(label: string, value: string | number | null | undefined): [string, string] {
		return [label, value === null || value === undefined || value === '' ? '—' : String(value)];
	}
</script>

<svelte:head><title>{m.security_login_events_detail_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` },
			{ name: m.nav_login_events(), path: `/tenants/${tenantId}/security/login-events` },
			{ name: m.security_login_events_detail_breadcrumb(), path: `/tenants/${tenantId}/security/login-events/${eventId}` }
		]
	}}
/>

<MainContent goBack={{ message: m.security_login_events_detail_back(), href: `/tenants/${tenantId}/security/login-events` }}>
	{#if detailQuery.isPending}
		<Skeleton class="h-64 w-full rounded-xl" />
	{:else if detailQuery.isError}
		<ErrorAlert title={m.security_login_events_detail_error_title()} message={detailQuery.error.message} />
	{:else}
		{@const d = detailQuery.data}
		{@const e = d.event}
		<div class="space-y-1">
			<h1 class="text-2xl font-bold tracking-tight">
				{m.security_login_events_detail_heading()} · <span class="{e.risk_level === 'CRITICAL' || e.risk_level === 'HIGH' ? 'text-red-600' : ''}">{e.risk_level}</span>
			</h1>
			<p class="text-muted-foreground text-sm">
				{m.security_login_events_detail_subtitle({ date: new Date(e.created_at).toLocaleString(), score: e.risk_score, decision: e.decision })}
			</p>
		</div>

		<div class="mt-6 grid gap-6 lg:grid-cols-2">
			<div class="rounded-xl border">
				<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_login_events_detail_section_event()}</div>
				<dl class="divide-y text-sm">
					{#each [ field(m.security_label_user(), e.username_normalized), field(m.security_login_events_detail_field_user_id(), e.user_id), field(m.security_login_events_detail_field_client_id(), e.client_id), field(m.security_login_events_detail_field_ip_address(), e.ip_address), field(m.security_label_country(), e.country_code), field(m.security_login_events_detail_field_city(), e.city), field(m.security_login_events_detail_field_region(), e.region), field(m.security_login_events_detail_field_asn(), e.asn), field(m.security_login_events_detail_field_isp(), e.isp), field(m.security_login_events_detail_field_user_agent(), e.user_agent), field(m.security_login_events_detail_field_credentials_valid(), e.credentials_valid ? m.security_yes() : m.security_no()), field(m.security_label_success(), e.success ? m.security_yes() : m.security_no()), field(m.security_login_events_detail_field_failure_reason(), e.failure_reason) ] as [label, value] (label)}
						<div class="flex justify-between gap-4 px-4 py-2">
							<dt class="text-muted-foreground">{label}</dt>
							<dd class="text-right font-mono text-xs break-all">{value}</dd>
						</div>
					{/each}
				</dl>
			</div>

			<div class="rounded-xl border">
				<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_login_events_detail_section_signals()}</div>
				<div class="space-y-3 p-4">
					{#each d.signals as s (s.signal_type)}
						<div class="rounded-lg border p-3">
							<div class="flex items-center justify-between">
								<span class="font-mono text-xs font-semibold">{s.signal_type}</span>
								<span class="text-muted-foreground text-xs">+{s.points} · {s.severity}</span>
							</div>
							<p class="text-muted-foreground mt-1 text-sm">{s.explanation}</p>
						</div>
					{/each}
					{#if d.signals.length === 0}
						<p class="text-muted-foreground py-4 text-center text-sm">{m.security_login_events_detail_no_signals()}</p>
					{/if}
				</div>
			</div>
		</div>

		{@render relatedTable(m.security_login_events_detail_related_by_ip(), d.related_by_ip)}
		{@render relatedTable(m.security_login_events_detail_related_by_user(), d.related_by_user)}
	{/if}
</MainContent>

{#snippet relatedTable(title: string, rows: LoginEventView[])}
	<div class="mt-6 rounded-xl border">
		<div class="border-b px-4 py-3 text-sm font-semibold">{title}</div>
		<div class="overflow-x-auto">
			<table class="w-full text-left text-sm">
				<thead class="bg-muted/50 text-muted-foreground">
					<tr>
						<th class="px-4 py-2 font-medium">{m.security_label_when()}</th>
						<th class="px-4 py-2 font-medium">{m.security_label_ip()}</th>
						<th class="px-4 py-2 font-medium">{m.security_label_country()}</th>
						<th class="px-4 py-2 font-medium">{m.security_label_success()}</th>
						<th class="px-4 py-2 font-medium">{m.security_label_risk()}</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as r (r.id)}
						<tr class="border-t">
							<td class="px-4 py-2 whitespace-nowrap">{new Date(r.created_at).toLocaleString()}</td>
							<td class="px-4 py-2 font-mono text-xs">{r.ip_address ?? '—'}</td>
							<td class="px-4 py-2">{r.country_code ?? '—'}</td>
							<td class="px-4 py-2">{r.success ? m.security_yes() : m.security_no()}</td>
							<td class="px-4 py-2">{r.risk_level}</td>
						</tr>
					{/each}
					{#if rows.length === 0}
						<tr><td colspan="5" class="text-muted-foreground px-4 py-6 text-center">{m.security_login_events_detail_related_empty()}</td></tr>
					{/if}
				</tbody>
			</table>
		</div>
	</div>
{/snippet}
