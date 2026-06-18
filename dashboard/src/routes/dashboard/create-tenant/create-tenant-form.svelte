<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import StrongPasswordDialog from './strong-password-dialog.svelte';
	import { createTenant, type TenantView } from '$lib/services/admin/tenants';
	import { getMyBilling } from '$lib/services/admin/subscriptions';
	import { planLabel } from '$lib/services/admin/subscription-plans';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';

	const queryClient = useQueryClient();

	let name = $state('');
	let description = $state('');
	let passwordHashSecret = $state('');
	let canSelfRegister = $state(false);
	let canSelfForgotPassword = $state(false);
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	// A new tenant always joins the admin's own organization's single subscription —
	// there is no cross-org picker. The plan's tenant cap is enforced server-side.
	const billingQuery = createQuery(() => ({
		queryKey: ['billing'],
		queryFn: async () => {
			const [data, err] = await getMyBilling();
			if (err) throw err;
			return data; // null when the org has no subscription yet
		}
	}));

	const billing = $derived(billingQuery.data ?? null);
	const subscriptionId = $derived(billing?.subscriptionId ?? '');
	const atTenantCap = $derived(!!billing && billing.usage.tenants >= billing.usage.maxTenants);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { subscriptionId, name, description, passwordHashSecret });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [tenant, err] = await createTenant({
			subscriptionId: result.data.subscriptionId,
			name: result.data.name.trim(),
			description: result.data.description.trim(),
			passwordHashSecret: result.data.passwordHashSecret,
			canSelfRegister,
			canSelfForgotPassword
		});

		isLoading = false;

		if (err) {
			console.error(err);
			toast.error(err.data?.message || 'An error occurred while creating the tenant.');
			return;
		}

		// Project the created tenant into the sidebar's list shape so the dropdown
		// reflects it immediately, without waiting for a refetch.
		const view: TenantView = {
			tenantId: tenant.tenant_id,
			name: tenant.name,
			description: tenant.description,
			isActive: tenant.is_active,
			canSelfRegister: tenant.can_self_register,
			canSelfForgotPassword: tenant.can_self_forgot_password,
			createdAt: tenant.created_at,
			updatedAt: null
		};

		// Optimistically add to the sidebar's ['tenants'] query cache (the dropdown
		// renders from it), then reconcile with the server. Also refresh billing so
		// the tenant-usage count updates.
		queryClient.setQueryData<TenantView[]>(['tenants'], (old) =>
			old ? [...old, view] : [view]
		);
		queryClient.invalidateQueries({ queryKey: ['tenants'] });
		queryClient.invalidateQueries({ queryKey: ['billing'] });

		// Make the new tenant the selected one. The store resolves `current` from the
		// URL's [tenantId] against its list, so seed the list (no duplicates) and
		// navigate to it — avoids an "Unknown tenant" flash before the query reconciles.
		if (!selectedTenant.tenants.some((t) => t.tenantId === view.tenantId)) {
			selectedTenant.tenants = [...selectedTenant.tenants, view];
		}

		toast.success('Tenant created successfully!');
		await goto(`/dashboard/${tenant.tenant_id}`);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-4">
	<div class="space-y-2">
		<Label>Subscription</Label>
		{#if billingQuery.isPending}
			<p class="flex items-center gap-2 text-sm text-muted-foreground">
				<LoadingSpinner /> Loading your subscription…
			</p>
		{:else if billing}
			<div class="rounded-md border bg-muted/30 p-3 text-sm">
				<p class="font-medium">{planLabel(billing.plan)}</p>
				<p class="text-muted-foreground">
					Using {billing.usage.tenants} of {billing.usage.maxTenants} tenants. The new tenant joins
					your organization's subscription.
				</p>
			</div>
			{#if atTenantCap}
				<p class="text-sm font-medium text-destructive">
					You've reached your plan's tenant limit. Upgrade your plan under
					<a class="underline" href="/dashboard/billing/plans">Billing</a> to add more.
				</p>
			{/if}
		{:else}
			<p class="text-sm text-muted-foreground">
				Your organization has no subscription yet.
				<a class="underline" href="/dashboard/billing/plans">Choose a plan</a> first — new tenants join
				it.
			</p>
		{/if}
		{#if errors.subscriptionId}
			<p class="text-sm font-medium text-destructive">{errors.subscriptionId}</p>
		{/if}
	</div>

	<Separator class="my-2" />

	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			Name
			<span class="text-red-500">*</span>
		</Label>
		<Input
			id="name"
			type="text"
			autocomplete="name"
			placeholder="Type the tenant name"
			bind:value={name}
		/>
		{#if errors.name}<p class="text-sm font-medium text-destructive">{errors.name}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description" class="flex gap-1">
			Description
			<span class="text-red-500">*</span>
		</Label>
		<Textarea id="description" placeholder="Type the tenant description" bind:value={description} />
		{#if errors.description}
			<p class="text-sm font-medium text-destructive">{errors.description}</p>
		{/if}
	</div>

	<Separator class="my-2" />

	<div class="space-y-2">
		<Label for="passwordHashSecret" class="flex gap-1">
			Password Hash Secret
			<span class="text-red-500">*</span>
		</Label>
		<Input
			id="passwordHashSecret"
			type="password"
			autocomplete="off"
			placeholder="Type the tenant password hash secret"
			bind:value={passwordHashSecret}
		/>

		<StrongPasswordDialog setPassword={(value) => (passwordHashSecret = value)} />

		<p class="text-sm text-muted-foreground">
			This is the secret that will be used to hash all the passwords from users that belong to this
			tenant.
		</p>
		{#if errors.passwordHashSecret}
			<p class="text-sm font-medium text-destructive">{errors.passwordHashSecret}</p>
		{/if}
	</div>

	<Separator class="my-2" />

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfRegister} />
		Allow users to self-register
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfForgotPassword} />
		Allow users to reset their own password
	</label>

	<Button type="submit" disabled={isLoading || !billing || atTenantCap} class="relative ml-auto w-fit">
		{#if isLoading}
			<LoadingSpinner class="absolute left-3" />
		{/if}
		{isLoading ? 'Adding Tenant...' : 'Add Tenant'}
	</Button>
</form>
