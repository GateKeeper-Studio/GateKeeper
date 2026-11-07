<script lang="ts">
	import { page } from '$app/state';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import {
		clearLoginEventLabel,
		getLoginEvent,
		labelLoginEvent,
		type LoginEventLabelView,
		type LoginEventView
	} from '$lib/services/admin/security';
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

	const queryClient = useQueryClient();
	let labeling = $state(false);

	// The admin's verdict: training feedback for the statistical model.
	async function setLabel(label: 'legitimate' | 'fraudulent' | null) {
		labeling = true;
		const err =
			label === null
				? (await clearLoginEventLabel(tenantId, eventId))[0]
				: (await labelLoginEvent(tenantId, eventId, { label }))[1];
		labeling = false;
		if (err) {
			toast.error(m.security_login_events_detail_label_error({ message: err.message }));
			return;
		}
		toast.success(m.security_login_events_detail_label_saved());
		queryClient.invalidateQueries({ queryKey: ['login-event', tenantId, eventId] });
	}

	function labelText(label: LoginEventLabelView): string {
		const value =
			label.label === 'fraudulent'
				? m.security_login_events_detail_label_value_fraudulent()
				: m.security_login_events_detail_label_value_legitimate();
		const source =
			label.source === 'admin'
				? m.security_login_events_detail_label_source_admin()
				: label.source === 'mfa_passed'
					? m.security_login_events_detail_label_source_mfa_passed()
					: m.security_login_events_detail_label_source_mfa_failed();
		return m.security_login_events_detail_label_current({ label: value, source });
	}

	const percent = (p: number) => Math.round(p * 100);

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

		<div class="mt-6 grid gap-6 lg:grid-cols-2">
			{#if d.risk_model}
				{@const r = d.risk_model}
				{@const scale = Math.max(...r.contributions.map((c) => Math.abs(c.contribution)), 0.0001)}
				<div class="rounded-xl border">
					<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_login_events_detail_section_model()}</div>
					<div class="space-y-3 p-4 text-sm">
						<p>
							{m.security_login_events_detail_model_summary({
								probability: percent(r.probability),
								threshold: percent(r.threshold),
								version: r.version,
								mode: r.mode
							})}
						</p>
						<p class={r.applied ? 'font-medium text-red-600' : 'text-muted-foreground'}>
							{r.applied
								? m.security_login_events_detail_model_applied()
								: r.would_escalate
									? m.security_login_events_detail_model_would_escalate()
									: m.security_login_events_detail_model_no_effect()}
						</p>
						<div>
							<p class="text-muted-foreground mb-2 text-xs">{m.security_login_events_detail_model_contributions()}</p>
							<ul class="space-y-1.5">
								{#each r.contributions as c (c.feature)}
									<li class="grid grid-cols-[10rem_1fr_3.5rem] items-center gap-2 text-xs">
										<span class="truncate font-mono" title={c.feature}>{c.feature}</span>
										<span class="bg-muted relative h-2 rounded">
											<span
												class="absolute top-0 h-2 rounded {c.contribution > 0 ? 'left-1/2 bg-red-500' : 'right-1/2 bg-emerald-500'}"
												style="width: {(Math.abs(c.contribution) / scale) * 50}%"
											></span>
										</span>
										<span class="text-right font-mono">{c.contribution > 0 ? '+' : ''}{c.contribution.toFixed(2)}</span>
									</li>
								{/each}
							</ul>
						</div>
					</div>
				</div>
			{/if}

			<div class="rounded-xl border">
				<div class="border-b px-4 py-3 text-sm font-semibold">{m.security_login_events_detail_section_label()}</div>
				<div class="space-y-3 p-4 text-sm">
					<p class="text-muted-foreground">{m.security_login_events_detail_label_description()}</p>
					<p class="font-medium">
						{d.label ? labelText(d.label) : m.security_login_events_detail_label_none()}
					</p>
					<div class="flex flex-wrap gap-2">
						<button
							class="rounded-md border px-3 py-1.5 text-sm disabled:opacity-50"
							disabled={labeling}
							onclick={() => setLabel('legitimate')}
						>
							{m.security_login_events_detail_label_mark_legitimate()}
						</button>
						<button
							class="rounded-md border border-red-300 px-3 py-1.5 text-sm text-red-700 disabled:opacity-50"
							disabled={labeling}
							onclick={() => setLabel('fraudulent')}
						>
							{m.security_login_events_detail_label_mark_fraudulent()}
						</button>
						{#if d.label}
							<button
								class="text-muted-foreground px-3 py-1.5 text-sm underline disabled:opacity-50"
								disabled={labeling}
								onclick={() => setLabel(null)}
							>
								{m.security_login_events_detail_label_clear()}
							</button>
						{/if}
					</div>
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
