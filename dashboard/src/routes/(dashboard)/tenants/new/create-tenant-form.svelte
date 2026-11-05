<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

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
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const queryClient = useQueryClient();

	let name = $state('');
	let description = $state('');
	let passwordHashSecret = $state('');
	let canSelfRegister = $state(false);
	let canSelfForgotPassword = $state(false);
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { name, description, passwordHashSecret });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [tenant, err] = await createTenant({
			name: result.data.name.trim(),
			description: result.data.description.trim(),
			passwordHashSecret: result.data.passwordHashSecret,
			canSelfRegister,
			canSelfForgotPassword
		});

		isLoading = false;

		if (err) {
			console.error(err);
			toast.error(err.data?.message || m.create_tenant_error());
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
			organizationId: tenant.organization_id,
			appearance: null,
			defaultLanguage: tenant.default_language,
			createdAt: tenant.created_at,
			updatedAt: null
		};

		// Optimistically add to the sidebar's ['tenants'] query cache (the dropdown
		// renders from it), then reconcile with the server.
		queryClient.setQueryData<TenantView[]>(['tenants'], (old) => (old ? [...old, view] : [view]));
		queryClient.invalidateQueries({ queryKey: ['tenants'] });

		// Make the new tenant the selected one. The store resolves `current` from the
		// URL's [tenantId] against its list, so seed the list (no duplicates) and
		// navigate to it — avoids an "Unknown tenant" flash before the query reconciles.
		if (!selectedTenant.tenants.some((t) => t.tenantId === view.tenantId)) {
			selectedTenant.tenants = [...selectedTenant.tenants, view];
		}

		toast.success(m.create_tenant_success());
		await goto(`/tenants/${tenant.tenant_id}`);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-4">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			{m.create_tenant_name_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input
			id="name"
			type="text"
			autocomplete="name"
			placeholder={m.create_tenant_name_placeholder()}
			bind:value={name}
		/>
		{#if errors.name}<p class="text-sm font-medium text-destructive">{errors.name}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description" class="flex gap-1">
			{m.create_tenant_description_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Textarea
			id="description"
			placeholder={m.create_tenant_description_placeholder()}
			bind:value={description}
		/>
		{#if errors.description}
			<p class="text-sm font-medium text-destructive">{errors.description}</p>
		{/if}
	</div>

	<Separator class="my-2" />

	<div class="space-y-2">
		<Label for="passwordHashSecret" class="flex gap-1">
			{m.create_tenant_password_hash_secret_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input
			id="passwordHashSecret"
			type="password"
			autocomplete="off"
			placeholder={m.create_tenant_password_hash_secret_placeholder()}
			bind:value={passwordHashSecret}
		/>

		<StrongPasswordDialog setPassword={(value) => (passwordHashSecret = value)} />

		<p class="text-sm text-muted-foreground">
			{m.create_tenant_password_hash_secret_description()}
		</p>
		{#if errors.passwordHashSecret}
			<p class="text-sm font-medium text-destructive">{errors.passwordHashSecret}</p>
		{/if}
	</div>

	<Separator class="my-2" />

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfRegister} />
		{m.create_tenant_allow_self_register()}
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfForgotPassword} />
		{m.create_tenant_allow_self_forgot_password()}
	</label>

	<Button type="submit" disabled={isLoading} class="relative ml-auto w-fit">
		{#if isLoading}
			<LoadingSpinner class="absolute left-3" />
		{/if}
		{isLoading ? m.create_tenant_submit_loading() : m.create_tenant_submit()}
	</Button>
</form>
