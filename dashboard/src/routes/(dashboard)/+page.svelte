<script lang="ts">
	import { goto } from '$app/navigation';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import { m } from '$lib/paraglide/messages.js';

	// The dashboard root (reached after login) auto-opens the remembered — or
	// first — tenant so the admin arrives on a usable page. `selectedTenant.tenantId`
	// already resolves to remembered-or-first, so we just follow it. `replaceState`
	// keeps the Back button from bouncing here. Only an account with zero tenants stays
	// on this page and sees the "create your first tenant" state.
	$effect(() => {
		if (!selectedTenant.loading && selectedTenant.tenants.length > 0 && selectedTenant.tenantId) {
			goto(`/tenants/${selectedTenant.tenantId}`, { replaceState: true });
		}
	});
</script>

<svelte:head>
	<title>{m.dashboard_home_meta_title()} - GateKeeper</title>
</svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [{ name: m.dashboard_home_breadcrumb(), path: '/' }]
	}}
/>

{#if selectedTenant.loading || selectedTenant.tenants.length > 0}
	<!-- Loading the list, or tenants exist and the effect above is redirecting into one. -->
	<div class="text-muted-foreground flex min-h-[60vh] items-center justify-center">
		<LoaderCircleIcon class="size-6 animate-spin" />
	</div>
{:else}
	<MainContent
		title={m.dashboard_home_title()}
		description={m.dashboard_home_description()}
	>
		<div
			class="mt-8 flex flex-col items-center justify-center rounded-xl border border-dashed p-12 text-center"
		>
			<div class="bg-muted mb-4 flex size-12 items-center justify-center rounded-xl">
				<Building2Icon class="size-6" />
			</div>
			<h2 class="text-lg font-semibold">{m.dashboard_home_empty_title()}</h2>
			<p class="text-muted-foreground mt-1 max-w-sm text-sm">
				{m.dashboard_home_empty_description()}
			</p>
			<Button href="/tenants/new" class="mt-6">
				<PlusIcon class="size-4" />
				{m.dashboard_home_create_tenant()}
			</Button>
		</div>
	</MainContent>
{/if}
