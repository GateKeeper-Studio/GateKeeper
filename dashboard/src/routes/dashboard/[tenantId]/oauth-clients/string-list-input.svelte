<script lang="ts">
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';

	// A small editor for a list of free-text strings (redirect URIs, custom scopes…).
	// Owns its own stable-keyed rows internally and writes the trimmed, non-empty
	// values back to the bindable `values` prop on every change.
	type Row = { id: number; value: string };

	type Props = {
		values: string[];
		placeholder?: string;
		addLabel?: string;
		inputmode?: 'text' | 'url';
	};

	let {
		values = $bindable(),
		placeholder = '',
		addLabel = 'Add',
		inputmode = 'text'
	}: Props = $props();

	let rows = $state<Row[]>(
		values.length ? values.map((value, i) => ({ id: i, value })) : [{ id: 0, value: '' }]
	);
	let nextId = rows.length;

	function sync() {
		values = rows.map((row) => row.value.trim()).filter(Boolean);
	}

	function add() {
		rows.push({ id: nextId++, value: '' });
	}

	function remove(id: number) {
		rows = rows.filter((row) => row.id !== id);
		if (rows.length === 0) rows.push({ id: nextId++, value: '' });
		sync();
	}
</script>

<div class="space-y-2">
	{#each rows as row (row.id)}
		<div class="flex items-center gap-2">
			<Input {placeholder} {inputmode} bind:value={row.value} oninput={sync} />
			<Button
				type="button"
				variant="ghost"
				size="icon"
				class="text-muted-foreground shrink-0"
				onclick={() => remove(row.id)}
				aria-label="Remove"
			>
				<XIcon />
			</Button>
		</div>
	{/each}

	<Button type="button" variant="outline" size="sm" onclick={add}>
		<PlusIcon />
		{addLabel}
	</Button>
</div>
