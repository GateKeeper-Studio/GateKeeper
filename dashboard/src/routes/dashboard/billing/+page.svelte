<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';

	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { getMyBilling } from '$lib/services/admin/subscriptions';
	import { formatPrice } from '$lib/services/admin/subscription-plans';

	const billingQuery = createQuery(() => ({
		queryKey: ['billing'],
		queryFn: async () => {
			const [data, err] = await getMyBilling();
			if (err) throw err;
			return data; // null when the org has no subscription yet
		}
	}));

	const billing = $derived(billingQuery.data ?? null);

	function pct(used: number, max: number): number {
		if (max <= 0) return 0;
		return Math.min(100, Math.round((used / max) * 100));
	}
</script>

<svelte:head><title>Billing - GateKeeper</title></svelte:head>

<DashboardHeader breadcrumbs={{ items: [{ name: 'Billing', path: '/dashboard/billing' }] }} />

<MainContent title="Billing" description="Your organization's subscription and usage.">
	{#if billingQuery.isPending}
		<div class="mt-6 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> Loading subscription…
		</div>
	{:else if billingQuery.isError}
		<ErrorAlert title="Couldn't load your subscription" message={billingQuery.error.message} />
	{:else if billing}
		<div class="mt-6 grid max-w-3xl gap-6">
			<Card.Root>
				<Card.Header>
					<div class="flex flex-wrap items-center justify-between gap-3">
						<div>
							<Card.Title class="text-xl">{billing.plan.name}</Card.Title>
							<Card.Description>
								{formatPrice(billing.plan.priceCents)} / {billing.plan.billingInterval}
							</Card.Description>
						</div>
						<div class="flex items-center gap-2">
							<Badge variant={billing.isActive ? 'default' : 'secondary'}>
								{billing.isActive ? 'Active' : 'Inactive'}
							</Badge>
							<Button href="/dashboard/billing/plans">
								<SparklesIcon /> Change plan
							</Button>
						</div>
					</div>
				</Card.Header>
				{#if billing.plan.description}
					<Card.Content class="text-sm text-muted-foreground">
						{billing.plan.description}
					</Card.Content>
				{/if}
			</Card.Root>

			<Card.Root>
				<Card.Header>
					<Card.Title>Usage</Card.Title>
					<Card.Description>How much of your plan you're currently using.</Card.Description>
				</Card.Header>
				<Card.Content class="grid gap-6">
					<div class="space-y-2">
						<div class="flex items-center justify-between text-sm">
							<span class="font-medium">Tenants</span>
							<span class="text-muted-foreground">
								{billing.usage.tenants} / {billing.usage.maxTenants}
							</span>
						</div>
						<div class="h-2 w-full overflow-hidden rounded-full bg-muted">
							<div
								class="h-full rounded-full bg-primary transition-all"
								style={`width: ${pct(billing.usage.tenants, billing.usage.maxTenants)}%`}
							></div>
						</div>
					</div>

					<div class="space-y-2">
						<div class="flex items-center justify-between text-sm">
							<span class="font-medium">Users in busiest tenant</span>
							<span class="text-muted-foreground">
								{billing.usage.maxUsersInAnyTenant} / {billing.usage.maxUsers}
							</span>
						</div>
						<div class="h-2 w-full overflow-hidden rounded-full bg-muted">
							<div
								class="h-full rounded-full bg-primary transition-all"
								style={`width: ${pct(billing.usage.maxUsersInAnyTenant, billing.usage.maxUsers)}%`}
							></div>
						</div>
						<p class="text-xs text-muted-foreground">Per-tenant limit, measured across all your tenants.</p>
					</div>
				</Card.Content>
			</Card.Root>
		</div>
	{:else}
		<Card.Root class="mt-6 max-w-3xl">
			<Card.Header>
				<Card.Title>No subscription yet</Card.Title>
				<Card.Description>
					Your organization hasn't contracted a plan. Choose one to start creating tenants and users.
				</Card.Description>
			</Card.Header>
			<Card.Content>
				<Button href="/dashboard/billing/plans">
					<CreditCardIcon /> Choose a plan
				</Button>
			</Card.Content>
		</Card.Root>
	{/if}
</MainContent>
