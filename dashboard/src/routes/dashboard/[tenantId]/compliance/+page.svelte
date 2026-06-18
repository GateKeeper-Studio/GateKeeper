<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { runPurge, type PurgeReport } from '$lib/services/admin/compliance';

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

<svelte:head><title>Compliance - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: 'Compliance', path: `/dashboard/${tenantId}/compliance` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">Compliance</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			Data-retention and governance controls (LGPD/GDPR/ISO 27001/SOC 2).
		</p>
	</div>

	<div class="mt-6 rounded-xl border p-4">
		<h3 class="text-lg font-semibold">Retention purge</h3>
		<p class="text-muted-foreground mt-1 text-sm">
			Delete expired refresh tokens, sessions, and authorization codes (storage limitation,
			GDPR Art. 5(1)(e)). The audit log is never purged. Runs across the platform.
		</p>
		<div class="mt-4">
			<Button onclick={handlePurge} disabled={purging}>
				{purging ? 'Purging…' : 'Run retention purge'}
			</Button>
		</div>

		{#if purgeError}
			<div class="mt-4">
				<ErrorAlert title="Purge failed" message={purgeError} />
			</div>
		{/if}

		{#if report}
			<div class="mt-4 grid grid-cols-2 gap-3 text-sm sm:grid-cols-4">
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">Refresh tokens</div>
					<div class="text-2xl font-semibold">{report.refresh_tokens}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">Sessions</div>
					<div class="text-2xl font-semibold">{report.auth_sessions}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">Auth codes</div>
					<div class="text-2xl font-semibold">{report.authorization_codes}</div>
				</div>
				<div class="rounded-lg border p-3">
					<div class="text-muted-foreground">Total</div>
					<div class="text-2xl font-semibold">{report.total}</div>
				</div>
			</div>
		{/if}
	</div>
</MainContent>
