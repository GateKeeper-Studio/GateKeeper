<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import ListChecksIcon from '@lucide/svelte/icons/list-checks';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import DynamicFieldsForm from '$lib/components/custom-fields/dynamic-fields-form.svelte';
	import { buildCustomFieldsZodSchema } from '$lib/components/custom-fields/schema';
	import type { CustomFieldDefinitionLike } from '$lib/components/custom-fields/types';

	import { listCustomFields, updateUserCustomFieldValues } from '$lib/services/admin/custom-fields';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import { validate } from '$lib/utils/form';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		userId: string;
		customFieldValues: Record<string, unknown> | null;
	};
	let { tenantId, userId, customFieldValues }: Props = $props();

	const queryClient = useQueryClient();
	const tenantDefaultLanguage = $derived(selectedTenant.current?.defaultLanguage ?? 'en');

	const fieldsQuery = createQuery(() => ({
		queryKey: ['custom-fields', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listCustomFields(tenantId);
			if (err) throw err;
			return data;
		}
	}));

	// Only active definitions are ever shown/saved — a retired field shouldn't
	// resurface just because the user has a stray stored value under its key.
	// Resolved down to the tenant's own default language: this admin editor
	// isn't locale-switchable, unlike the sign-up-form preview.
	const activeDefinitions = $derived<CustomFieldDefinitionLike[]>(
		(fieldsQuery.data ?? [])
			.filter((def) => def.isActive)
			.map((def) => ({
				fieldKey: def.fieldKey,
				label: resolveLocalized(def.label, tenantDefaultLanguage, tenantDefaultLanguage),
				description:
					resolveLocalized(def.description, tenantDefaultLanguage, tenantDefaultLanguage) || null,
				fieldType: def.fieldType,
				options:
					def.options?.map((o) =>
						resolveLocalized(o, tenantDefaultLanguage, tenantDefaultLanguage)
					) ?? null,
				isRequired: def.isRequired,
				displayOrder: def.displayOrder
			}))
	);

	let values = $state<Record<string, unknown>>(customFieldValues ?? {});
	let errors = $state<Record<string, string>>({});
	let saving = $state(false);

	async function handleSave() {
		const result = validate(buildCustomFieldsZodSchema(activeDefinitions), values);
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		saving = true;

		const [data, err] = await updateUserCustomFieldValues(tenantId, userId, result.data);

		saving = false;

		if (err) {
			toast.error(err.data?.message || m.users_custom_fields_save_error());
			return;
		}

		values = data?.customFieldValues ?? {};
		toast.success(m.users_custom_fields_save_success());
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId, userId] });
	}
</script>

<div class="rounded-xl border p-4">
	<h3 class="text-lg font-semibold">{m.users_custom_fields_title()}</h3>
	<p class="mt-1 text-sm text-muted-foreground">{m.users_custom_fields_description()}</p>

	<div class="mt-4">
		{#if fieldsQuery.isPending}
			<Skeleton class="h-24 w-full rounded-lg" />
		{:else if fieldsQuery.isError}
			<ErrorAlert
				title={m.users_custom_fields_load_error_title()}
				message={fieldsQuery.error.message}
			/>
		{:else if activeDefinitions.length === 0}
			<div
				class="flex flex-col items-center gap-2 rounded-lg border border-dashed p-8 text-center text-muted-foreground"
			>
				<ListChecksIcon class="size-7 opacity-60" />
				<p class="text-sm">{m.users_custom_fields_empty()}</p>
			</div>
		{:else}
			<DynamicFieldsForm definitions={activeDefinitions} bind:values {errors} />
			<Button class="mt-4" onclick={handleSave} disabled={saving}>
				{saving ? m.users_custom_fields_saving() : m.users_custom_fields_save_button()}
			</Button>
		{/if}
	</div>
</div>
