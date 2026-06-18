<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { updateMyOrganization, type OrganizationView } from '$lib/services/admin/organizations';

	let { organization }: { organization: OrganizationView } = $props();

	const queryClient = useQueryClient();

	let name = $state(organization.name);
	let description = $state(organization.description ?? '');
	let isActive = $state(organization.isActive);

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

		const [, err] = await updateMyOrganization({
			name: name.trim(),
			description: description.trim() || null,
			isActive
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || err.message || 'Could not update the organization.');
			return;
		}

		toast.success('Organization updated');
		queryClient.invalidateQueries({ queryKey: ['organization'] });
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-6">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			Name
			<span class="text-red-500">*</span>
		</Label>
		<Input id="name" type="text" placeholder="Acme Inc" bind:value={name} />
		{#if error}<p class="text-destructive text-sm font-medium">{error}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description">Description</Label>
		<Input
			id="description"
			type="text"
			placeholder="Optional description"
			bind:value={description}
		/>
	</div>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={isActive} />
		Active
		<span class="text-muted-foreground">— inactive organizations are kept but flagged disabled.</span>
	</label>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="relative w-fit">
			{#if isLoading}<LoadingSpinner class="absolute left-3" />{/if}
			{isLoading ? 'Saving…' : 'Save changes'}
		</Button>
	</div>
</form>
