<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SaveIcon from '@lucide/svelte/icons/save';
	import { m } from '$lib/paraglide/messages.js';

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
			error = m.settings_organization_name_required();
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
			toast.error(err.data?.message || err.message || m.settings_organization_update_error());
			return;
		}

		toast.success(m.settings_organization_update_success());
		queryClient.invalidateQueries({ queryKey: ['organization'] });
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-6">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			{m.settings_organization_name_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input id="name" type="text" placeholder={m.settings_organization_name_placeholder()} bind:value={name} />
		{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description">{m.settings_organization_description_label()}</Label>
		<Input
			id="description"
			type="text"
			placeholder={m.settings_organization_description_placeholder()}
			bind:value={description}
		/>
	</div>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={isActive} />
		{m.settings_organization_active_label()}
		<span class="text-muted-foreground"
			>{m.settings_organization_active_helper()}</span
		>
	</label>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="w-fit">
			{#if isLoading}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
			{isLoading ? m.settings_organization_saving() : m.settings_organization_save_button()}
		</Button>
	</div>
</form>
