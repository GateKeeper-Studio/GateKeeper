<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { m } from '$lib/paraglide/messages.js';
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
	const listHref = `/tenants/${tenantId}/groups`;

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
				toast.error(err.data?.message || m.groups_create_error_toast());
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
			toast.success(m.groups_create_success_toast({ name: data.name }));
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
			toast.error(err.data?.message || m.groups_update_error_toast());
			return;
		}

		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId, group!.group_id] });
		toast.success(m.groups_update_success_toast());
		await goto(listHref);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-6">
	<div class="grid gap-4">
		<div class="flex items-center gap-2">
			<UsersRoundIcon class="text-muted-foreground size-4" />
			<span class="text-sm font-medium">{m.groups_form_identity_section()}</span>
		</div>

		<div class="space-y-2">
			<Label for="name" class="flex gap-1">
				{m.groups_form_name_label()}
				<span class="text-red-500">*</span>
			</Label>
			<Input
				id="name"
				type="text"
				placeholder={m.groups_form_name_placeholder()}
				bind:value={name}
			/>
			{#if errors.name}
				<p class="text-destructive text-sm font-medium">{errors.name}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<div class="flex items-center gap-2">
				<AlignLeftIcon class="text-muted-foreground size-4" />
				<Label for="description">{m.groups_form_description_label()}</Label>
			</div>
			<Textarea
				id="description"
				placeholder={m.groups_form_description_placeholder()}
				rows={3}
				bind:value={description}
			/>
			{#if errors.description}
				<p class="text-destructive text-sm font-medium">{errors.description}</p>
			{/if}
		</div>
	</div>

	<div class="flex justify-end gap-2">
		<Button type="button" variant="outline" href={listHref}>{m.groups_cancel()}</Button>
		<Button type="submit" disabled={isLoading}>
			{#if isLoading}
				<LoadingSpinner class="size-4" />
			{:else if mode === 'edit'}
				<SaveIcon />
			{/if}
			{#if mode === 'create'}
				{isLoading ? m.groups_form_creating_loading() : m.groups_create_group_title()}
			{:else}
				{isLoading ? m.groups_form_saving_loading() : m.groups_save_changes()}
			{/if}
		</Button>
	</div>
</form>
