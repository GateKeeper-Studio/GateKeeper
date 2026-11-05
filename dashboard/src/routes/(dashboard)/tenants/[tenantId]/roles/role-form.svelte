<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import ShieldIcon from '@lucide/svelte/icons/shield';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import {
		createTenantRole,
		updateTenantRole,
		type TenantRoleView
	} from '$lib/services/admin/roles';
	import { roleFormSchema } from './schema.js';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		mode: 'create' | 'edit';
		tenantId: string;
		role?: TenantRoleView;
	};

	let { mode, tenantId, role }: Props = $props();

	const queryClient = useQueryClient();
	const listHref = `/tenants/${tenantId}/roles`;

	let name = $state(role?.name ?? '');
	let description = $state(role?.description ?? '');

	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(roleFormSchema, {
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
			const [data, err] = await createTenantRole(tenantId, {
				name: result.data.name,
				description: descriptionValue
			});
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || m.roles_form_create_error());
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });
			toast.success(m.roles_form_create_success({ name: data.name }));
			await goto(listHref);
			return;
		}

		// edit mode
		const [err] = await updateTenantRole(tenantId, role!.role_id, {
			name: result.data.name,
			description: descriptionValue
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || m.roles_form_update_error());
			return;
		}

		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId, role!.role_id] });
		toast.success(m.roles_form_update_success());
		await goto(listHref);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-6">
	<div class="grid gap-4">
		<div class="flex items-center gap-2">
			<ShieldIcon class="text-muted-foreground size-4" />
			<span class="text-sm font-medium">{m.roles_form_identity_section()}</span>
		</div>

		<div class="space-y-2">
			<Label for="name" class="flex gap-1">
				{m.roles_form_name_label()}
				<span class="text-red-500">*</span>
			</Label>
			<Input
				id="name"
				type="text"
				placeholder={m.roles_form_name_placeholder()}
				bind:value={name}
			/>
			{#if errors.name}
				<p class="text-destructive text-sm font-medium">{errors.name}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<div class="flex items-center gap-2">
				<AlignLeftIcon class="text-muted-foreground size-4" />
				<Label for="description">{m.roles_form_description_label()}</Label>
			</div>
			<Textarea
				id="description"
				placeholder={m.roles_form_description_placeholder()}
				rows={3}
				bind:value={description}
			/>
			{#if errors.description}
				<p class="text-destructive text-sm font-medium">{errors.description}</p>
			{/if}
		</div>
	</div>

	<div class="flex justify-end gap-2">
		<Button type="button" variant="outline" href={listHref}>{m.roles_action_cancel()}</Button>
		<Button type="submit" disabled={isLoading}>
			{#if isLoading}
				<LoadingSpinner class="size-4" />
			{:else if mode === 'edit'}
				<SaveIcon />
			{/if}
			{#if mode === 'create'}
				{isLoading ? m.roles_form_creating_loading() : m.roles_create_role_title()}
			{:else}
				{isLoading ? m.roles_form_saving_loading() : m.roles_action_save_changes()}
			{/if}
		</Button>
	</div>
</form>
