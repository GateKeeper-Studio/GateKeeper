<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import {
		createTenantGroup,
		updateTenantGroup,
		type TenantGroupView
	} from '$lib/services/admin/groups';
	import { groupFormSchema } from './schema.js';

	type Props = {
		mode: 'create' | 'edit';
		tenantId: string;
		group?: TenantGroupView;
	};

	let { mode, tenantId, group }: Props = $props();

	const queryClient = useQueryClient();
	const listHref = `/dashboard/${tenantId}/groups`;

	let name = $state(group?.name ?? '');
	let description = $state(group?.description ?? '');

	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(groupFormSchema, {
			name: name.trim(),
			description: description.trim()
		});

		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const descriptionValue = result.data.description?.trim() || null;

		if (mode === 'create') {
			const [data, err] = await createTenantGroup(tenantId, {
				name: result.data.name,
				description: descriptionValue
			});
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || 'An error occurred while creating the group.');
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
			toast.success(`Group "${data.name}" created`);
			await goto(listHref);
			return;
		}

		// edit mode
		const [err] = await updateTenantGroup(tenantId, group!.group_id, {
			name: result.data.name,
			description: descriptionValue
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || 'An error occurred while updating the group.');
			return;
		}

		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId, group!.group_id] });
		toast.success('Group updated');
		await goto(listHref);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-6">
	<div class="grid gap-4">
		<div class="flex items-center gap-2">
			<UsersRoundIcon class="text-muted-foreground size-4" />
			<span class="text-sm font-medium">Identity</span>
		</div>

		<div class="space-y-2">
			<Label for="name" class="flex gap-1">
				Name
				<span class="text-red-500">*</span>
			</Label>
			<Input
				id="name"
				type="text"
				placeholder="e.g. Admins, Engineering, Support"
				bind:value={name}
			/>
			{#if errors.name}
				<p class="text-destructive text-sm font-medium">{errors.name}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<div class="flex items-center gap-2">
				<AlignLeftIcon class="text-muted-foreground size-4" />
				<Label for="description">Description</Label>
			</div>
			<Textarea
				id="description"
				placeholder="What this group represents or who belongs to it"
				rows={3}
				bind:value={description}
			/>
			{#if errors.description}
				<p class="text-destructive text-sm font-medium">{errors.description}</p>
			{/if}
		</div>
	</div>

	<div class="flex justify-end gap-2">
		<Button type="button" variant="outline" href={listHref}>Cancel</Button>
		<Button type="submit" disabled={isLoading} class="relative">
			{#if isLoading}<LoadingSpinner />{/if}
			{#if mode === 'create'}
				{isLoading ? 'Creating group…' : 'Create group'}
			{:else}
				{isLoading ? 'Saving changes…' : 'Save changes'}
			{/if}
		</Button>
	</div>
</form>
