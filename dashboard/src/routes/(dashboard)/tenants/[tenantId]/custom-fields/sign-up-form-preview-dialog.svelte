<!--
  Self-contained mock of the real hosted sign-up form (default fields + this
  tenant's active custom fields, in order) — not an iframe of the real page.
  Nothing here is ever submitted to the backend; "Validate" only runs the same
  client-side Zod schemas the real sign-up form uses, for admin sanity-checking.
-->
<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import DynamicFieldsForm from '$lib/components/custom-fields/dynamic-fields-form.svelte';
	import { buildCustomFieldsZodSchema } from '$lib/components/custom-fields/schema';
	import type { CustomFieldDefinitionLike } from '$lib/components/custom-fields/types';

	import { validate } from '$lib/utils/form';
	import { formSchema } from '../../../../auth/[clientId]/sign-up/schema';
	import type { CustomFieldDefinitionView } from '$lib/services/admin/custom-fields';
	import { LOCALES } from '$lib/i18n/locales';
	import { resolveLocalized } from '$lib/i18n/resolve-localized';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		open: boolean;
		definitions: CustomFieldDefinitionView[];
		defaultLanguage: string;
	};

	let { open = $bindable(false), definitions, defaultLanguage }: Props = $props();

	// Which language this preview is currently resolving definitions into — a
	// tenant-wide tool, so it offers every supported locale rather than being
	// scoped to any one client's allowed languages.
	let previewLocale = $state(defaultLanguage);

	let displayName = $state('');
	let firstName = $state('');
	let lastName = $state('');
	let email = $state('');
	let password = $state('');
	let customFieldValues = $state<Record<string, unknown>>({});

	let defaultErrors = $state<Record<string, string>>({});
	let customFieldErrors = $state<Record<string, string>>({});

	// Mirrors auth-form.svelte's assembly order: default fields first, active
	// custom fields appended after password, sorted the same way
	// dynamic-fields-form.svelte defensively re-sorts them. Each definition's
	// per-language label/description/options are resolved down to the
	// currently selected preview locale — the renderer (DynamicFieldsForm)
	// only ever deals in single resolved strings, same as the real hosted page.
	const activeDefinitions = $derived<CustomFieldDefinitionLike[]>(
		definitions
			.filter((definition) => definition.isActive)
			.sort((a, b) => a.displayOrder - b.displayOrder)
			.map((definition) => ({
				fieldKey: definition.fieldKey,
				label: resolveLocalized(definition.label, previewLocale, defaultLanguage),
				description: resolveLocalized(definition.description, previewLocale, defaultLanguage) || null,
				fieldType: definition.fieldType,
				options:
					definition.options?.map((o) => resolveLocalized(o, previewLocale, defaultLanguage)) ??
					null,
				isRequired: definition.isRequired,
				displayOrder: definition.displayOrder
			}))
	);

	function runValidation() {
		const result = validate(formSchema, { displayName, firstName, lastName, email, password });
		defaultErrors = result.success ? {} : result.errors;

		if (activeDefinitions.length > 0) {
			const customResult = validate(
				buildCustomFieldsZodSchema(activeDefinitions),
				customFieldValues
			);
			customFieldErrors = customResult.success ? {} : customResult.errors;
		} else {
			customFieldErrors = {};
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="max-h-[85vh] overflow-y-auto sm:max-w-2xl">
		<Dialog.Header class="flex flex-row items-start justify-between gap-4 pr-8">
			<div>
				<Dialog.Title>{m.custom_fields_preview_dialog_title()}</Dialog.Title>
				<Dialog.Description>{m.custom_fields_preview_disclaimer()}</Dialog.Description>
			</div>
			<div class="space-y-1">
				<Label class="text-xs">{m.custom_fields_preview_language_label()}</Label>
				<Select.Root
					type="single"
					value={previewLocale}
					onValueChange={(v) => v && (previewLocale = v)}
				>
					<Select.Trigger class="h-8 w-36 text-xs">
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
			</div>
		</Dialog.Header>

		<div class="grid gap-4">
			<div class="space-y-2">
				<Label for="preview-displayName">{m.signup_display_name_label()}</Label>
				<Input
					id="preview-displayName"
					type="text"
					autocomplete="name"
					placeholder={m.signup_display_name_placeholder()}
					bind:value={displayName}
				/>
				{#if defaultErrors.displayName}
					<p class="text-sm font-medium text-destructive">{defaultErrors.displayName}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="preview-firstName">{m.signup_first_name_label()}</Label>
				<Input
					id="preview-firstName"
					type="text"
					autocomplete="given-name"
					placeholder={m.signup_first_name_placeholder()}
					bind:value={firstName}
				/>
				{#if defaultErrors.firstName}
					<p class="text-sm font-medium text-destructive">{defaultErrors.firstName}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="preview-lastName">{m.signup_last_name_label()}</Label>
				<Input
					id="preview-lastName"
					type="text"
					autocomplete="family-name"
					placeholder={m.signup_last_name_placeholder()}
					bind:value={lastName}
				/>
				{#if defaultErrors.lastName}
					<p class="text-sm font-medium text-destructive">{defaultErrors.lastName}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="preview-email">{m.auth_email_label()}</Label>
				<Input
					id="preview-email"
					type="email"
					autocomplete="email"
					placeholder={m.auth_email_placeholder()}
					bind:value={email}
				/>
				{#if defaultErrors.email}
					<p class="text-sm font-medium text-destructive">{defaultErrors.email}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="preview-password">{m.auth_password_label()}</Label>
				<Input
					id="preview-password"
					type="password"
					autocomplete="new-password"
					placeholder="********"
					bind:value={password}
				/>
				{#if defaultErrors.password}
					<p class="text-sm font-medium text-destructive">{defaultErrors.password}</p>
				{/if}
			</div>

			{#if activeDefinitions.length > 0}
				<DynamicFieldsForm
					definitions={activeDefinitions}
					bind:values={customFieldValues}
					errors={customFieldErrors}
				/>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={runValidation}>
				{m.custom_fields_preview_validate_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
