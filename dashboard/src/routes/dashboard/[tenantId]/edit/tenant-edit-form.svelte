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

	import { updateTenant, type TenantDetail } from '$lib/services/admin/tenants';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';

	let { tenant }: { tenant: TenantDetail } = $props();

	const queryClient = useQueryClient();

	let name = $state(tenant.name);
	let description = $state(tenant.description ?? '');
	let isActive = $state(tenant.is_active);
	let canSelfRegister = $state(tenant.can_self_register);
	let canSelfForgotPassword = $state(tenant.can_self_forgot_password);

	let error = $state('');
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (name.trim().length === 0) {
			error = 'Name cannot be empty';
			return;
		}
		error = '';
		isLoading = true;

		const [err] = await updateTenant(tenant.tenant_id, {
			name: name.trim(),
			description: description.trim() || null,
			isActive,
			canSelfRegister,
			canSelfForgotPassword
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || err.message || 'Could not update the tenant.');
			return;
		}

		// Reflect the new name/status in the sidebar list right away.
		selectedTenant.tenants = selectedTenant.tenants.map((t) =>
			t.tenantId === tenant.tenant_id
				? {
						...t,
						name: name.trim(),
						description: description.trim() || null,
						isActive,
						canSelfRegister,
						canSelfForgotPassword
					}
				: t
		);
		queryClient.invalidateQueries({ queryKey: ['tenant', tenant.tenant_id] });
		queryClient.invalidateQueries({ queryKey: ['tenants'] });

		toast.success('Tenant updated');
		await goto(`/dashboard/${tenant.tenant_id}`);
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-4">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			Name
			<span class="text-red-500">*</span>
		</Label>
		<Input id="name" type="text" placeholder="Tenant name" bind:value={name} />
		{#if error}<p class="text-destructive text-sm font-medium">{error}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description">Description</Label>
		<Textarea id="description" placeholder="Optional description" bind:value={description} />
	</div>

	<Separator class="my-2" />

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={isActive} />
		Active
		<span class="text-muted-foreground">— inactive tenants are kept but flagged disabled.</span>
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfRegister} />
		Allow users to self-register
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfForgotPassword} />
		Allow users to reset their own password
	</label>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="relative w-fit">
			{#if isLoading}<LoadingSpinner class="absolute left-3" />{/if}
			{isLoading ? 'Saving…' : 'Save changes'}
		</Button>
	</div>
</form>
