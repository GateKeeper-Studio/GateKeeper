<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { listSuspiciousIps, blockIp, unblockIp } from '$lib/services/admin/security';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const queryClient = useQueryClient();

	let busy = $state<string | null>(null);

	const ipsQuery = createQuery(() => ({
		queryKey: ['suspicious-ips', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listSuspiciousIps(tenantId, { limit: 100 });
			if (err) throw err;
			return data.items;
		}
	}));

	async function doBlock(ip: string) {
		busy = ip;
		const [, err] = await blockIp(tenantId, { ip_address: ip, reason: 'Blocked from dashboard' });
		busy = null;
		if (err) {
			toast.error(m.security_suspicious_ips_block_error({ ip, message: err.message }));
			return;
		}
		toast.success(m.security_suspicious_ips_block_success({ ip }));
		queryClient.invalidateQueries({ queryKey: ['suspicious-ips', tenantId] });
	}

	async function doUnblock(ip: string, blockId: string) {
		busy = ip;
		const [err] = await unblockIp(tenantId, blockId);
		busy = null;
		if (err) {
			toast.error(m.security_suspicious_ips_unblock_error({ ip, message: err.message }));
			return;
		}
		toast.success(m.security_suspicious_ips_unblock_success({ ip }));
		queryClient.invalidateQueries({ queryKey: ['suspicious-ips', tenantId] });
	}
</script>

<svelte:head><title>{m.security_suspicious_ips_page_title()}</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_security(), path: `/tenants/${tenantId}/security` },
			{ name: m.nav_suspicious_ips(), path: `/tenants/${tenantId}/security/suspicious-ips` }
		]
	}}
/>

<MainContent>
	<div class="space-y-1">
		<h1 class="text-3xl font-bold tracking-tight">{m.nav_suspicious_ips()}</h1>
		<p class="text-muted-foreground max-w-2xl text-sm">
			{m.security_suspicious_ips_description()}
		</p>
	</div>

	<div class="mt-6">
		{#if ipsQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if ipsQuery.isError}
			<ErrorAlert title={m.security_suspicious_ips_error_title()} message={ipsQuery.error.message} />
		{:else}
			<div class="overflow-x-auto rounded-xl border">
				<table class="w-full text-left text-sm">
					<thead class="bg-muted/50 text-muted-foreground">
						<tr>
							<th class="px-4 py-2 font-medium">{m.security_label_ip()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_country()}</th>
							<th class="px-4 py-2 font-medium">{m.security_suspicious_ips_col_attempts()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_failed()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_users()}</th>
							<th class="px-4 py-2 font-medium">{m.security_label_max_score()}</th>
							<th class="px-4 py-2 font-medium">{m.security_suspicious_ips_col_status()}</th>
							<th class="px-4 py-2 text-right font-medium">{m.security_suspicious_ips_col_action()}</th>
						</tr>
					</thead>
					<tbody>
						{#each ipsQuery.data as ip (ip.ip_address)}
							<tr class="border-t">
								<td class="px-4 py-2 font-mono text-xs">{ip.ip_address}</td>
								<td class="px-4 py-2">{ip.country_code ?? '—'}</td>
								<td class="px-4 py-2">{ip.attempts}</td>
								<td class="px-4 py-2">{ip.failed_attempts}</td>
								<td class="px-4 py-2">{ip.distinct_users}</td>
								<td class="px-4 py-2">{ip.max_risk_score}</td>
								<td class="px-4 py-2">
									{#if ip.blocked}
										<span class="rounded bg-red-100 px-2 py-0.5 text-xs font-medium text-red-800 dark:bg-red-950 dark:text-red-300">{m.security_suspicious_ips_status_blocked()}</span>
									{:else}
										<span class="text-muted-foreground text-xs">{m.security_suspicious_ips_status_active()}</span>
									{/if}
								</td>
								<td class="px-4 py-2 text-right">
									{#if ip.blocked && ip.block_id}
										<button
											class="border-input rounded-md border px-3 py-1 text-xs disabled:opacity-50"
											disabled={busy === ip.ip_address}
											onclick={() => doUnblock(ip.ip_address, ip.block_id!)}
										>
											{m.security_suspicious_ips_unblock_button()}
										</button>
									{:else}
										<button
											class="rounded-md border border-red-300 px-3 py-1 text-xs text-red-700 disabled:opacity-50 dark:border-red-800 dark:text-red-300"
											disabled={busy === ip.ip_address}
											onclick={() => doBlock(ip.ip_address)}
										>
											{m.security_suspicious_ips_block_button()}
										</button>
									{/if}
								</td>
							</tr>
						{/each}
						{#if ipsQuery.data.length === 0}
							<tr><td colspan="8" class="text-muted-foreground px-4 py-8 text-center">{m.security_no_suspicious_ips()}</td></tr>
						{/if}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</MainContent>
