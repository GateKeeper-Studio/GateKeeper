<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';

	import CheckIcon from '@lucide/svelte/icons/check';
	import BuildingIcon from '@lucide/svelte/icons/building';
	import UsersIcon from '@lucide/svelte/icons/users';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import {
		listSubscriptionPlans,
		formatPrice,
		type SubscriptionPlanView
	} from '$lib/services/admin/subscription-plans';
	import { getMyBilling, changePlan, contractSubscription } from '$lib/services/admin/subscriptions';

	const queryClient = useQueryClient();

	const plansQuery = createQuery(() => ({
		queryKey: ['subscription-plans'],
		queryFn: async () => {
			const [data, err] = await listSubscriptionPlans();
			if (err) throw err;
			return data.items.filter((plan) => plan.isActive);
		}
	}));

	const billingQuery = createQuery(() => ({
		queryKey: ['billing'],
		queryFn: async () => {
			const [data, err] = await getMyBilling();
			if (err) throw err;
			return data; // null when the org has no subscription yet
		}
	}));

	const billing = $derived(billingQuery.data ?? null);

	// Billing interval toggle (monthly / yearly). Default to whatever the current
	// subscription is on once it loads, so the active plan is visible by default.
	let interval = $state<'monthly' | 'yearly'>('monthly');
	let intervalInitialized = false;
	$effect(() => {
		if (!intervalInitialized && billing) {
			interval = billing.plan.billingInterval === 'yearly' ? 'yearly' : 'monthly';
			intervalInitialized = true;
		}
	});

	const hasYearly = $derived((plansQuery.data ?? []).some((p) => p.billingInterval === 'yearly'));
	const hasMonthly = $derived((plansQuery.data ?? []).some((p) => p.billingInterval === 'monthly'));

	const visiblePlans = $derived(
		(plansQuery.data ?? [])
			.filter((plan) => plan.billingInterval === interval)
			.sort((a, b) => a.priceCents - b.priceCents)
	);

	// Highlight a "Popular" plan: the median-priced option in the current interval.
	const recommendedId = $derived(
		visiblePlans.length > 0 ? visiblePlans[Math.floor(visiblePlans.length / 2)].planId : null
	);

	// A plan the current usage cannot fit (a downgrade) — returns the reason, or null.
	function downgradeReason(plan: SubscriptionPlanView): string | null {
		if (!billing) return null;
		if (plan.maxTenants < billing.usage.tenants) {
			return `Your organization has ${billing.usage.tenants} tenant(s); this plan allows ${plan.maxTenants}. Remove tenants first.`;
		}
		if (plan.maxUsers < billing.usage.maxUsersInAnyTenant) {
			return `A tenant has ${billing.usage.maxUsersInAnyTenant} user(s); this plan allows ${plan.maxUsers} per tenant.`;
		}
		return null;
	}

	let selected = $state<SubscriptionPlanView | null>(null);
	let confirmOpen = $state(false);
	let submitting = $state(false);

	function openConfirm(plan: SubscriptionPlanView) {
		selected = plan;
		confirmOpen = true;
	}

	async function confirmSwitch() {
		if (!selected) return;
		submitting = true;
		const [, err] = billing
			? await changePlan(selected.planId, billing.isActive)
			: await contractSubscription(selected.planId);
		submitting = false;

		if (err) {
			toast.error(err.data?.message || err.data?.error || err.message);
			return;
		}

		toast.success(billing ? 'Plan updated' : 'Subscription started');
		confirmOpen = false;
		queryClient.invalidateQueries({ queryKey: ['billing'] });
		queryClient.invalidateQueries({ queryKey: ['subscription-plans'] });
		await goto('/dashboard/billing');
	}
</script>

<svelte:head><title>Change plan - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: 'Billing', path: '/dashboard/billing' },
			{ name: 'Plans', path: '/dashboard/billing/plans' }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to billing', href: '/dashboard/billing' }}
	title={billing ? 'Change your plan' : 'Choose a plan'}
	description="Pick the plan that fits your organization. Switch any time."
>
	{#if plansQuery.isPending}
		<div class="mt-6 flex items-center gap-2 text-muted-foreground">
			<LoadingSpinner /> Loading plans…
		</div>
	{:else if plansQuery.isError}
		<ErrorAlert title="Couldn't load plans" message={plansQuery.error.message} />
	{:else}
		<!-- Billing interval toggle -->
		{#if hasMonthly && hasYearly}
			<div class="mt-6 inline-flex w-fit rounded-lg border bg-muted/40 p-1 text-sm">
				<button
					type="button"
					class={`rounded-md px-4 py-1.5 font-medium transition-colors ${interval === 'monthly' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}
					onclick={() => (interval = 'monthly')}
				>
					Monthly
				</button>
				<button
					type="button"
					class={`rounded-md px-4 py-1.5 font-medium transition-colors ${interval === 'yearly' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}
					onclick={() => (interval = 'yearly')}
				>
					Yearly
				</button>
			</div>
		{/if}

		{#if visiblePlans.length === 0}
			<p class="mt-6 text-muted-foreground">No {interval} plans are available right now.</p>
		{:else}
			<div class="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each visiblePlans as plan (plan.planId)}
					{@const isCurrent = billing?.plan.planId === plan.planId}
					{@const recommended = plan.planId === recommendedId}
					{@const blocked = downgradeReason(plan)}
					<Card.Root
						class={`relative flex flex-col ${recommended ? 'border-primary shadow-md ring-1 ring-primary' : ''}`}
					>
						{#if recommended}
							<Badge class="absolute -top-0 left-1/2 -translate-x-1/2 rounded-t-none px-4">Popular</Badge>
						{/if}
						<Card.Header>
							<Card.Title class="text-lg">{plan.name}</Card.Title>
							{#if plan.description}
								<Card.Description>{plan.description}</Card.Description>
							{/if}
							<div class="mt-2 flex items-baseline gap-1">
								<span class="text-3xl font-bold tracking-tight">{formatPrice(plan.priceCents)}</span>
								<span class="text-sm text-muted-foreground">/{plan.billingInterval}</span>
							</div>
						</Card.Header>
						<Card.Content class="flex-1">
							<ul class="grid gap-2 text-sm">
								<li class="flex items-center gap-2">
									<BuildingIcon size={16} class="text-muted-foreground" />
									Up to <strong>{plan.maxTenants}</strong> tenant{plan.maxTenants === 1 ? '' : 's'}
								</li>
								<li class="flex items-center gap-2">
									<UsersIcon size={16} class="text-muted-foreground" />
									Up to <strong>{plan.maxUsers}</strong> users per tenant
								</li>
							</ul>
						</Card.Content>
						<Card.Footer class="flex-col items-stretch gap-2">
							{#if isCurrent}
								<Button variant="outline" disabled class="w-full">
									<CheckIcon /> Current plan
								</Button>
							{:else if blocked}
								<Button variant="outline" disabled class="w-full">Not available</Button>
								<p class="text-xs text-destructive">{blocked}</p>
							{:else}
								<Button
									variant={recommended ? 'default' : 'outline'}
									class="w-full"
									onclick={() => openConfirm(plan)}
								>
									{billing ? 'Switch to this plan' : 'Choose plan'}
								</Button>
							{/if}
						</Card.Footer>
					</Card.Root>
				{/each}
			</div>
		{/if}
	{/if}
</MainContent>

<Dialog.Root bind:open={confirmOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>
				{billing ? 'Switch to' : 'Start'} {selected?.name}?
			</Dialog.Title>
			<Dialog.Description>
				{#if selected}
					{#if billing}
						Your subscription will move to <strong>{selected.name}</strong> ({formatPrice(
							selected.priceCents
						)}/{selected.billingInterval}) effective immediately.
					{:else}
						This contracts the <strong>{selected.name}</strong> plan ({formatPrice(
							selected.priceCents
						)}/{selected.billingInterval}) for your organization and provisions your first tenant.
					{/if}
				{/if}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={submitting} onclick={() => (confirmOpen = false)}>
				Cancel
			</Button>
			<Button disabled={submitting} onclick={confirmSwitch}>
				{#if submitting}<LoadingSpinner />{/if}
				{billing ? 'Confirm switch' : 'Start subscription'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
