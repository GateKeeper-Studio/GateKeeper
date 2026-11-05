<!--
  Security: definition-derived text (label/description) renders exclusively via
  `{}` interpolation — never `{@html ...}`. See dynamic-field-input.svelte for the
  full rationale; the same rule applies to every string rendered in this file.
-->
<script lang="ts">
	import { Label } from '$lib/components/ui/label/index.js';
	import DynamicFieldInput from './dynamic-field-input.svelte';
	import type { CustomFieldDefinitionLike } from './types';

	type Props = {
		definitions: CustomFieldDefinitionLike[];
		values: Record<string, unknown>;
		errors?: Record<string, string>;
		selectPlaceholder?: string;
	};

	let { definitions, values = $bindable(), errors = {}, selectPlaceholder = '' }: Props = $props();

	const sorted = $derived(
		[...definitions].sort((a, b) => (a.displayOrder ?? 0) - (b.displayOrder ?? 0))
	);
</script>

{#if sorted.length > 0}
	<div class="grid gap-4">
		{#each sorted as def (def.fieldKey)}
			<div class="space-y-2">
				<Label for={def.fieldKey} class="flex gap-1">
					{def.label}
					{#if def.isRequired}<span class="text-red-500">*</span>{/if}
				</Label>
				{#if def.description}
					<p class="text-sm text-muted-foreground">{def.description}</p>
				{/if}
				<DynamicFieldInput
					definition={def}
					bind:value={values[def.fieldKey]}
					error={errors[def.fieldKey]}
					placeholder={selectPlaceholder}
				/>
				{#if errors[def.fieldKey]}
					<p class="text-sm font-medium text-destructive">{errors[def.fieldKey]}</p>
				{/if}
			</div>
		{/each}
	</div>
{/if}
