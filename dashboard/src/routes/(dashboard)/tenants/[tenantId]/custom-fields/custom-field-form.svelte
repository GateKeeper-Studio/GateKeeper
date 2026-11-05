<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';
	import SaveIcon from '@lucide/svelte/icons/save';
	import PlusIcon from '@lucide/svelte/icons/plus';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import DynamicFieldInput from '$lib/components/custom-fields/dynamic-field-input.svelte';

	import { validate } from '$lib/utils/form';
	import {
		createCustomField,
		updateCustomField,
		listCustomFields,
		type CustomFieldDefinitionView,
		type CustomFieldType
	} from '$lib/services/admin/custom-fields';
	import StringListInput from '../oauth-clients/string-list-input.svelte';
	import { buildCustomFieldFormSchema } from './schema.js';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { LOCALES } from '$lib/i18n/locales';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		mode: 'create' | 'edit';
		tenantId: string;
		field?: CustomFieldDefinitionView;
	};

	let { mode, tenantId, field }: Props = $props();

	const queryClient = useQueryClient();

	const TYPE_OPTIONS: { value: CustomFieldType; label: string }[] = [
		{ value: 'text', label: m.custom_fields_type_text() },
		{ value: 'number', label: m.custom_fields_type_number() },
		{ value: 'boolean', label: m.custom_fields_type_boolean() },
		{ value: 'date', label: m.custom_fields_type_date() },
		{ value: 'select', label: m.custom_fields_type_select() }
	];

	// Every custom field's label must cover this locale (server-enforced too) — it's
	// also the initial language tab and the initial preview locale.
	const defaultLanguage = $derived(selectedTenant.current?.defaultLanguage ?? 'en');

	// ---- form state (seeded from `field` in edit mode) -----------------------
	let fieldKey = $state(field?.fieldKey ?? '');
	let label = $state<Record<string, string>>({ ...(field?.label ?? {}) });
	let description = $state<Record<string, string>>({ ...(field?.description ?? {}) });
	let fieldType = $state<CustomFieldType>(field?.fieldType ?? 'text');
	let optionRows = $state<{ id: number; values: Record<string, string> }[]>(
		(field?.options ?? []).map((values, i) => ({ id: i, values: { ...values } }))
	);
	let isRequired = $state(field?.isRequired ?? false);
	let isActive = $state(field?.isActive ?? true);

	// Which language tab is being edited (label/description/options), and which
	// locale the live preview resolves against — independent controls, both
	// starting on the tenant's own default language.
	let activeLocale = $state(defaultLanguage);
	let previewLocale = $state(defaultLanguage);

	// Local scratch value for the live preview card — never submitted anywhere.
	let previewValue = $state<unknown>(undefined);

	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	const typeLabel = $derived(TYPE_OPTIONS.find((t) => t.value === fieldType)?.label ?? '');

	const previewLabel = $derived(resolveLocalized(label, previewLocale, defaultLanguage));
	const previewDescription = $derived(
		resolveLocalized(description, previewLocale, defaultLanguage)
	);
	const previewOptions = $derived(
		optionRows.map((row) => resolveLocalized(row.values, previewLocale, defaultLanguage))
	);

	function dropEmpty(map: Record<string, string>): Record<string, string> {
		const cleaned: Record<string, string> = {};
		for (const [locale, value] of Object.entries(map)) {
			const trimmed = value.trim();
			if (trimmed) cleaned[locale] = trimmed;
		}
		return cleaned;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const schema = buildCustomFieldFormSchema(defaultLanguage);
		const result = validate(schema, {
			fieldKey: fieldKey.trim(),
			label,
			description,
			fieldType,
			options: fieldType === 'select' ? optionRows.map((r) => r.values) : [],
			isRequired,
			isActive
		});

		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const cleanedLabel = dropEmpty(result.data.label);
		const cleanedDescription = dropEmpty(result.data.description);
		const cleanedOptions =
			result.data.fieldType === 'select' ? result.data.options.map((o) => dropEmpty(o)) : null;

		if (mode === 'create') {
			// Ordering is now owned entirely by the list page's drag/arrow UI — a new
			// field is always appended after the current last one.
			const [existing, listErr] = await listCustomFields(tenantId);
			if (listErr) {
				isLoading = false;
				toast.error(listErr.data?.message || m.custom_fields_form_create_error());
				return;
			}
			const nextOrder = existing.length ? Math.max(...existing.map((f) => f.displayOrder)) + 1 : 0;

			const [, err] = await createCustomField(tenantId, {
				fieldKey: result.data.fieldKey,
				label: cleanedLabel,
				description: Object.keys(cleanedDescription).length ? cleanedDescription : null,
				fieldType: result.data.fieldType,
				options: cleanedOptions,
				isRequired: result.data.isRequired,
				displayOrder: nextOrder
			});
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || m.custom_fields_form_create_error());
				return;
			}

			toast.success(m.custom_fields_form_create_success());
			queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId] });
			await goto(resolve('/(dashboard)/tenants/[tenantId]/custom-fields', { tenantId }));
			return;
		}

		const [, err] = await updateCustomField(tenantId, field!.id, {
			label: cleanedLabel,
			description: Object.keys(cleanedDescription).length ? cleanedDescription : null,
			options: cleanedOptions,
			isRequired: result.data.isRequired,
			// Reordering happens exclusively on the list page now — keep this field's
			// existing position untouched on any other edit.
			displayOrder: field!.displayOrder,
			isActive: result.data.isActive
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || m.custom_fields_form_update_error());
			return;
		}

		toast.success(m.custom_fields_form_update_success());
		queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId, field!.id] });
		await goto(resolve('/(dashboard)/tenants/[tenantId]/custom-fields', { tenantId }));
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
	<div class="grid gap-6">
		<Card.Root>
			<Card.Header>
				<Card.Title>{m.custom_fields_form_section_identity()}</Card.Title>
			</Card.Header>
			<Card.Content class="grid gap-4">
				<div class="space-y-2">
					<Label for="fieldKey" class="flex gap-1">
						{m.custom_fields_form_key_label()}
						{#if mode === 'create'}<span class="text-red-500">*</span>{/if}
					</Label>

					{#if mode === 'create'}
						<Input
							id="fieldKey"
							type="text"
							placeholder={m.custom_fields_form_key_placeholder()}
							bind:value={fieldKey}
						/>
						<p class="text-sm text-muted-foreground">{m.custom_fields_form_key_hint()}</p>
						{#if errors.fieldKey}<p class="text-sm font-medium text-destructive">
								{errors.fieldKey}
							</p>{/if}
					{:else}
						<p class="text-sm">
							<code class="rounded bg-muted px-1.5 py-0.5 font-mono">{fieldKey}</code>
							<span class="text-muted-foreground">{m.custom_fields_form_key_locked_hint()}</span>
						</p>
					{/if}
				</div>

				<Tabs.Root
					value={activeLocale}
					onValueChange={(v) => v && (activeLocale = v)}
				>
					<Tabs.List>
						{#each LOCALES as locale (locale.code)}
							<Tabs.Trigger value={locale.code}>
								{locale.nativeLabel}
								{#if locale.code === defaultLanguage}<span class="text-red-500">*</span>{/if}
							</Tabs.Trigger>
						{/each}
					</Tabs.List>

					{#each LOCALES as locale (locale.code)}
						<Tabs.Content value={locale.code} class="grid gap-4">
							<div class="space-y-2">
								<Label for="label-{locale.code}" class="flex gap-1">
									{m.custom_fields_form_label_label()}
									{#if locale.code === defaultLanguage}<span class="text-red-500">*</span>{/if}
								</Label>
								<Input
									id="label-{locale.code}"
									type="text"
									placeholder={m.custom_fields_form_label_placeholder()}
									bind:value={label[locale.code]}
								/>
								{#if locale.code === defaultLanguage && errors.label}
									<p class="text-sm font-medium text-destructive">{errors.label}</p>
								{/if}
							</div>

							<div class="space-y-2">
								<Label for="description-{locale.code}"
									>{m.custom_fields_form_description_label()}</Label
								>
								<Textarea
									id="description-{locale.code}"
									placeholder={m.custom_fields_form_description_placeholder()}
									bind:value={description[locale.code]}
								/>
								{#if locale.code === defaultLanguage && errors.description}
									<p class="text-sm font-medium text-destructive">{errors.description}</p>
								{/if}
							</div>
						</Tabs.Content>
					{/each}
				</Tabs.Root>
				<p class="text-sm text-muted-foreground">
					{m.custom_fields_form_language_tab_required_hint()}
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>{m.custom_fields_form_section_type()}</Card.Title>
			</Card.Header>
			<Card.Content class="grid gap-4">
				<div class="space-y-2">
					<Label class="flex gap-1">
						{m.custom_fields_form_type_label()}
						{#if mode === 'create'}<span class="text-red-500">*</span>{/if}
					</Label>

					{#if mode === 'create'}
						<Select.Root type="single" bind:value={fieldType}>
							<Select.Trigger class="w-full">{typeLabel}</Select.Trigger>
							<Select.Content>
								{#each TYPE_OPTIONS as option (option.value)}
									<Select.Item value={option.value} label={option.label}
										>{option.label}</Select.Item
									>
								{/each}
							</Select.Content>
						</Select.Root>
					{:else}
						<p class="text-sm">
							{typeLabel}
							<span class="text-muted-foreground">{m.custom_fields_form_type_locked_hint()}</span>
						</p>
					{/if}
				</div>

				{#if fieldType === 'select'}
					<div class="space-y-2">
						<Label class="flex gap-1">
							{m.custom_fields_form_options_label()}
							<span class="text-red-500">*</span>
						</Label>
						<p class="text-xs text-muted-foreground">
							{m.custom_fields_form_options_language_hint({
								language: LOCALES.find((l) => l.code === activeLocale)?.nativeLabel ?? activeLocale
							})}
						</p>
						<StringListInput
							bind:entries={optionRows}
							locale={activeLocale}
							addLabel={m.custom_fields_form_add_option()}
						/>
						{#if errors.options}<p class="text-sm font-medium text-destructive">
								{errors.options}
							</p>{/if}
					</div>
				{/if}
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>{m.custom_fields_form_section_behavior()}</Card.Title>
			</Card.Header>
			<Card.Content class="grid gap-3">
				<label class="flex items-center gap-2 text-sm">
					<Checkbox bind:checked={isRequired} />
					{m.custom_fields_form_required_label()}
					<span class="text-muted-foreground">{m.custom_fields_form_required_description()}</span>
				</label>

				{#if mode === 'edit'}
					<label class="flex items-center gap-2 text-sm">
						<Checkbox bind:checked={isActive} />
						{m.custom_fields_form_active_label()}
						<span class="text-muted-foreground">{m.custom_fields_form_active_description()}</span>
					</label>
				{/if}
			</Card.Content>
		</Card.Root>

		{#if mode === 'create' && fieldType === 'select' && errors.options}
			<Alert.Root variant="destructive">
				<Alert.Description>{errors.options}</Alert.Description>
			</Alert.Root>
		{/if}

		<div class="flex justify-end">
			<Button type="submit" disabled={isLoading} class="w-fit">
				{#if isLoading}
					<LoadingSpinner class="size-4" />
				{:else if mode === 'create'}
					<PlusIcon />
				{:else}
					<SaveIcon />
				{/if}
				{#if mode === 'create'}
					{isLoading ? m.custom_fields_form_creating_field() : m.custom_fields_form_create_field()}
				{:else}
					{isLoading ? m.custom_fields_form_saving() : m.custom_fields_form_save_changes()}
				{/if}
			</Button>
		</div>
	</div>

	<Card.Root class="h-fit lg:sticky lg:top-4">
		<Card.Header class="flex flex-row items-center justify-between gap-2">
			<Card.Title>{m.custom_fields_form_preview_title()}</Card.Title>
			<Select.Root
				type="single"
				value={previewLocale}
				onValueChange={(v) => v && (previewLocale = v)}
			>
				<Select.Trigger class="h-8 w-28 text-xs">
					{LOCALES.find((l) => l.code === previewLocale)?.nativeLabel ?? previewLocale}
				</Select.Trigger>
				<Select.Content>
					{#each LOCALES as locale (locale.code)}
						<Select.Item value={locale.code} label={locale.nativeLabel}
							>{locale.nativeLabel}</Select.Item
						>
					{/each}
				</Select.Content>
			</Select.Root>
		</Card.Header>
		<Card.Content class="space-y-2">
			<Label for="preview-field" class="flex gap-1">
				{previewLabel || m.custom_fields_form_preview_placeholder_label()}
				{#if isRequired}<span class="text-red-500">*</span>{/if}
			</Label>
			{#if previewDescription}
				<p class="text-sm text-muted-foreground">{previewDescription}</p>
			{/if}
			<DynamicFieldInput
				definition={{
					fieldKey: 'preview-field',
					label: previewLabel,
					description: previewDescription,
					fieldType,
					options: previewOptions,
					isRequired
				}}
				bind:value={previewValue}
			/>
		</Card.Content>
	</Card.Root>
</form>
