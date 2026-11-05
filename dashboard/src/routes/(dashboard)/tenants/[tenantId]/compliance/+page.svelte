<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { runPurge, type PurgeReport } from '$lib/services/admin/compliance';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);

	let purging = $state(false);
	let report = $state<PurgeReport | null>(null);
	let purgeError = $state<string | null>(null);

	async function handlePurge() {
		purgeError = null;
		report = null;
		purging = true;
		const [data, err] = await runPurge();
		purging = false;
		if (err) {
			purgeError = err.message;
			return;
		}
		report = data;
	}
</script>

<svelte:head><title>{m.nav_compliance()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_compliance(), path: `/tenants/${tenantId}/compliance` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_compliance()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.tenant_compliance_description()}
		</p>
	</div>

	<div class="mt-6 rounded-xl border p-4">
		<h3 class="text-lg font-semibold">{m.tenant_retention_purge_title()}</h3>
		<p class="text-muted-foreground mt-1 text-sm">
			{m.tenant_retention_purge_description()}
		</p>
		<div class="mt-4">
			<Button onclick={handlePurge} disabled={purging}>
				{purging ? m.tenant_purging() : m.tenant_run_purge()}
			</Button>
		</div>

		{#if purgeError}
			<div class="mt-4">
				<ErrorAlert title={m.tenant_purge_failed_title()} message={purgeError} />
			</div>
		{/if}

		{#if report}
			<div class="mt-4 grid grid-cols-2 gap-3 text-sm sm:grid-cols-4">
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">{m.tenant_report_refresh_tokens()}</div>
					<div class="text-2xl font-semibold">{report.refresh_tokens}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">{m.tenant_report_sessions()}</div>
					<div class="text-2xl font-semibold">{report.auth_sessions}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">{m.tenant_report_auth_codes()}</div>
					<div class="text-2xl font-semibold">{report.authorization_codes}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">{m.tenant_report_total()}</div>
					<div class="text-2xl font-semibold">{report.total}</div>
				</div>
			</div>
		{/if}
	</div>
</MainContent>
