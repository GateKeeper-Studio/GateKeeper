<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import { dndzone, type DndEvent } from 'svelte-dnd-action';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import ListChecksIcon from '@lucide/svelte/icons/list-checks';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import {
		listCustomFields,
		reorderCustomFields,
		type CustomFieldDefinitionView
	} from '$lib/services/admin/custom-fields';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import DeleteCustomFieldDialog from './delete-custom-field-dialog.svelte';
	import SignUpFormPreviewDialog from './sign-up-form-preview-dialog.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const tenantDefaultLanguage = $derived(selectedTenant.current?.defaultLanguage ?? 'en');
	const queryClient = useQueryClient();

	// Unpaged — the API hard-caps this at 40 fields per tenant, so the whole
	// list is always fetched at once (see list_tenant_custom_fields on the API).
	const fieldsQuery = createQuery(() => ({
		queryKey: ['custom-fields', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listCustomFields(tenantId);
			if (err) throw err;
			return [...data].sort((a, b) => a.displayOrder - b.displayOrder);
		}
	}));

	// Local, drag/arrow-mutable copy of the server order — resynced from the
	// query whenever it (re)loads (initial fetch, or after a reorder/CRUD
	// invalidates it). Mutated optimistically by drag/arrow actions, then
	// persisted through `persistOrder`.
	let orderedFields = $state<CustomFieldDefinitionView[]>([]);

	$effect(() => {
		if (fieldsQuery.data) {
			orderedFields = fieldsQuery.data;
		}
	});

	function typeLabel(type: CustomFieldDefinitionView['fieldType']): string {
		switch (type) {
			case 'text':
				return m.custom_fields_type_text();
			case 'number':
				return m.custom_fields_type_number();
			case 'boolean':
				return m.custom_fields_type_boolean();
			case 'date':
				return m.custom_fields_type_date();
			case 'select':
				return m.custom_fields_type_select();
		}
	}

	async function persistOrder(newOrder: CustomFieldDefinitionView[]) {
		const previous = orderedFields;
		orderedFields = newOrder;

		const [err] = await reorderCustomFields(
			tenantId,
			newOrder.map((field) => field.id)
		);

		if (err) {
			toast.error(err.data?.message || m.custom_fields_reorder_error());
			orderedFields = previous;
			queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId] });
			return;
		}

		toast.success(m.custom_fields_reorder_success());
		queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId] });
	}

	function handleConsider(e: CustomEvent<DndEvent<CustomFieldDefinitionView>>) {
		orderedFields = e.detail.items;
	}

	function handleFinalize(e: CustomEvent<DndEvent<CustomFieldDefinitionView>>) {
		void persistOrder(e.detail.items);
	}

	function moveUp(index: number) {
		if (index <= 0) return;
		const next = [...orderedFields];
		[next[index - 1], next[index]] = [next[index], next[index - 1]];
		void persistOrder(next);
	}

	function moveDown(index: number) {
		if (index >= orderedFields.length - 1) return;
		const next = [...orderedFields];
		[next[index], next[index + 1]] = [next[index + 1], next[index]];
		void persistOrder(next);
	}

	let fieldToDelete = $state<CustomFieldDefinitionView | null>(null);
	let previewOpen = $state(false);
</script>

<svelte:head><title>{m.custom_fields_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.custom_fields_title(), path: `/tenants/${tenantId}/custom-fields` }
		]
	}}
/>

<MainContent>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">{m.custom_fields_title()}</h1>
			<p class="max-w-2xl text-sm text-muted-foreground">
				{m.custom_fields_page_description()}
			</p>
		</div>

		<div class="flex gap-2">
			<Button
				variant="outline"
				onclick={() => (previewOpen = true)}
				disabled={fieldsQuery.isPending || fieldsQuery.isError}
			>
				<EyeIcon />
				{m.custom_fields_preview_button()}
			</Button>

			<Button href={`/tenants/${tenantId}/custom-fields/new`}>
				<PlusIcon />
				{m.custom_fields_new()}
			</Button>
		</div>
	</div>

	<div class="mt-6">
		{#if fieldsQuery.isPending}
			<Skeleton class="h-64 w-full rounded-xl" />
		{:else if fieldsQuery.isError}
			<ErrorAlert title={m.custom_fields_load_error_title()} message={fieldsQuery.error.message} />
		{:else if orderedFields.length === 0}
			<div class="flex flex-col items-center rounded-4xl border border-dashed p-12 text-center">
				<ListChecksIcon class="mb-3 size-8 text-muted-foreground" />
				<p class="font-medium">{m.custom_fields_empty_title()}</p>
				<p class="mt-1 text-sm text-muted-foreground">{m.custom_fields_empty_description()}</p>
			</div>
		{:else}
			<div class="overflow-hidden rounded-xl border">
				<div class="flex items-center gap-3 bg-muted/40 px-4 py-2 text-sm font-medium">
					<div class="w-14 shrink-0" aria-hidden="true"></div>
					<div class="min-w-0 flex-1">{m.custom_fields_table_label()}</div>
					<div class="hidden w-32 shrink-0 font-mono text-xs sm:block">
						{m.custom_fields_table_key()}
					</div>
					<div class="w-28 shrink-0">{m.custom_fields_table_type()}</div>
					<div class="hidden w-20 shrink-0 sm:block">{m.custom_fields_table_required()}</div>
					<div class="hidden w-20 shrink-0 sm:block">{m.custom_fields_table_active()}</div>
					<div class="w-20 shrink-0"></div>
				</div>

				<div
					class="flex flex-col divide-y"
					use:dndzone={{ items: orderedFields, flipDurationMs: 200 }}
					onconsider={handleConsider}
					onfinalize={handleFinalize}
				>
					{#each orderedFields as fieldDef, index (fieldDef.id)}
						<div
							class={[
								'flex items-center gap-3 bg-background px-4 py-3',
								!fieldDef.isActive && 'opacity-60'
							]}
						>
							<div class="flex w-14 shrink-0 items-center gap-1">
								<span
									class="cursor-grab text-muted-foreground active:cursor-grabbing"
									aria-label={m.custom_fields_reorder_drag_handle()}
								>
									<GripVerticalIcon class="size-4" />
								</span>
								<div class="flex flex-col">
									<button
										type="button"
										class="text-muted-foreground hover:text-foreground disabled:opacity-30"
										disabled={index === 0}
										onclick={() => moveUp(index)}
										aria-label={m.custom_fields_reorder_move_up()}
									>
										<ChevronUpIcon class="size-3.5" />
									</button>
									<button
										type="button"
										class="text-muted-foreground hover:text-foreground disabled:opacity-30"
										disabled={index === orderedFields.length - 1}
										onclick={() => moveDown(index)}
										aria-label={m.custom_fields_reorder_move_down()}
									>
										<ChevronDownIcon class="size-3.5" />
									</button>
								</div>
							</div>

							<div class="min-w-0 flex-1 font-medium">
								{resolveLocalized(fieldDef.label, tenantDefaultLanguage, tenantDefaultLanguage)}
							</div>

							<div
								class="hidden w-32 shrink-0 truncate font-mono text-xs text-muted-foreground sm:block"
							>
								{fieldDef.fieldKey}
							</div>

							<div class="w-28 shrink-0">
								<Badge variant="secondary">{typeLabel(fieldDef.fieldType)}</Badge>
							</div>

							<div class="hidden w-20 shrink-0 text-sm sm:block">
								{fieldDef.isRequired ? m.custom_fields_yes() : m.custom_fields_no()}
							</div>

							<div class="hidden w-20 shrink-0 text-sm sm:block">
								{fieldDef.isActive ? m.custom_fields_yes() : m.custom_fields_no()}
							</div>

							<div class="flex w-20 shrink-0 justify-end gap-1">
								<Button
									variant="ghost"
									size="icon"
									href={`/tenants/${tenantId}/custom-fields/${fieldDef.id}/edit`}
									aria-label={m.custom_fields_action_edit()}
								>
									<PencilIcon class="size-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									class="text-destructive hover:text-destructive"
									onclick={() => (fieldToDelete = fieldDef)}
									aria-label={m.custom_fields_action_delete()}
								>
									<Trash2Icon class="size-4" />
								</Button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</MainContent>

<DeleteCustomFieldDialog
	{tenantId}
	field={fieldToDelete}
	onOpenChange={() => (fieldToDelete = null)}
/>

<SignUpFormPreviewDialog
	bind:open={previewOpen}
	definitions={orderedFields}
	defaultLanguage={tenantDefaultLanguage}
/>
