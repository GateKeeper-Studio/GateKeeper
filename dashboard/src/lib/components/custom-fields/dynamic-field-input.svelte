<!--
  Security: every definition-derived string (label/description/option) and the
  bound `value` render exclusively via `{}` text interpolation below — never
  `{@html ...}`. That is the entire XSS defense for tenant-authored/user-submitted
  custom-field content: no field type here is rich text, and this codebase has no
  HTML-sanitizer dependency to fall back on.
-->
<script lang="ts">
	import { Input } from '$lib/components/ui/input/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import type { CustomFieldDefinitionLike } from './types';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		definition: CustomFieldDefinitionLike;
		value?: unknown;
		error?: string;
		placeholder?: string;
	};

	let { definition, value = $bindable(undefined), error, placeholder = '' }: Props = $props();

	// Falls back to a type-phrased hint built from the field's own label when
	// the caller doesn't supply a placeholder — every text/number/date/select
	// input always shows one. Lowercased since it sits mid-sentence in the hint.
	const lowerLabel = $derived(definition.label.toLowerCase());
	const effectivePlaceholder = $derived(
		placeholder ||
			{
				text: m.custom_field_placeholder_text({ label: lowerLabel }),
				number: m.custom_field_placeholder_number({ label: lowerLabel }),
				boolean: '',
				date: m.custom_field_placeholder_date({ label: lowerLabel }),
				select: m.custom_field_placeholder_select({ label: lowerLabel })
			}[definition.fieldType]
	);
</script>

{#if definition.fieldType === 'text'}
	<Input
		id={definition.fieldKey}
		type="text"
		maxlength={1000}
		placeholder={effectivePlaceholder}
		aria-invalid={!!error}
		bind:value={() => (value as string | undefined) ?? '', (v) => (value = v)}
	/>
{:else if definition.fieldType === 'number'}
	<Input
		id={definition.fieldKey}
		type="number"
		placeholder={effectivePlaceholder}
		aria-invalid={!!error}
		bind:value={() => value as number | undefined, (v) => (value = v)}
	/>
{:else if definition.fieldType === 'boolean'}
	<Checkbox
		id={definition.fieldKey}
		bind:checked={() => (value as boolean | undefined) ?? false, (v) => (value = v)}
	/>
{:else if definition.fieldType === 'date'}
	<Input
		id={definition.fieldKey}
		type="date"
		placeholder={effectivePlaceholder}
		aria-invalid={!!error}
		bind:value={() => (value as string | undefined) ?? '', (v) => (value = v)}
	/>
{:else if definition.fieldType === 'select'}
	<Select.Root
		type="single"
		value={(value as string | undefined) ?? ''}
		onValueChange={(v) => (value = v || undefined)}
	>
		<Select.Trigger id={definition.fieldKey} class="w-full" aria-invalid={!!error}>
			{(value as string | undefined) || effectivePlaceholder}
		</Select.Trigger>
		<Select.Content>
			{#each definition.options ?? [] as option (option)}
				<Select.Item value={option} label={option}>{option}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
{/if}
