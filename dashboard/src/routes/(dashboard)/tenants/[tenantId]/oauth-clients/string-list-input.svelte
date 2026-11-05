<script lang="ts">
	import { dndzone, type DndEvent } from 'svelte-dnd-action';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { m } from '$lib/paraglide/messages.js';

	// A small editor for a list of free-text strings (redirect URIs, custom scopes…)
	// OR — when the caller supplies `locale` + `entries` instead of `values` — a list
	// of per-language "choice" objects (localized select options). Either way there is
	// exactly one row array being reordered, so every language's translation of a
	// choice stays index-aligned: dragging / moving / adding / removing a row moves
	// that choice's text in every language at once, since a row already carries every
	// language's text for that choice — there's nothing to keep in sync separately.
	//
	// Rows can be reordered by dragging (mouse/touch, via svelte-dnd-action) or with
	// the up/down arrow buttons (keyboard-accessible fallback) — both paths funnel
	// through `sync()` so the bound prop always reflects the on-screen order.
	type FlatRow = { id: number; value: string };
	type LocalizedRow = { id: number; values: Record<string, string> };
	type AnyRow = FlatRow | LocalizedRow;

	type Props = {
		values?: string[];
		entries?: LocalizedRow[];
		locale?: string;
		placeholder?: string;
		addLabel?: string;
		inputmode?: 'text' | 'url';
	};

	let {
		values = $bindable(),
		entries = $bindable(),
		locale,
		placeholder = '',
		addLabel = 'Add',
		inputmode = 'text'
	}: Props = $props();

	const localized = locale !== undefined;

	function blankRow(id: number): AnyRow {
		return localized ? { id, values: {} } : { id, value: '' };
	}

	let rows = $state<AnyRow[]>(
		localized
			? entries!.length
				? entries!
				: [blankRow(0)]
			: values!.length
				? values!.map((value, i) => ({ id: i, value }))
				: [blankRow(0)]
	);
	let nextId = rows.length;

	function textOf(row: AnyRow): string {
		return localized ? ((row as LocalizedRow).values[locale!] ?? '') : (row as FlatRow).value;
	}

	function setText(row: AnyRow, text: string) {
		if (localized) {
			(row as LocalizedRow).values[locale!] = text;
		} else {
			(row as FlatRow).value = text;
		}
	}

	// Flat mode: trims + drops empties into the bound `values` array (unchanged
	// behavior). Localized mode: the row array itself IS every language's data —
	// no filtering here (a value empty in the *current* tab may still hold text in
	// another language), just hand the live array back to the bound `entries` prop.
	function sync() {
		if (localized) {
			entries = rows as LocalizedRow[];
		} else {
			values = (rows as FlatRow[]).map((row) => row.value.trim()).filter(Boolean);
		}
	}

	function add() {
		rows.push(blankRow(nextId++));
		sync();
	}

	function remove(id: number) {
		rows = rows.filter((row) => row.id !== id);
		if (rows.length === 0) rows.push(blankRow(nextId++));
		sync();
	}

	function moveUp(index: number) {
		if (index <= 0) return;
		[rows[index - 1], rows[index]] = [rows[index], rows[index - 1]];
		sync();
	}

	function moveDown(index: number) {
		if (index >= rows.length - 1) return;
		[rows[index], rows[index + 1]] = [rows[index + 1], rows[index]];
		sync();
	}

	function handleDndConsider(e: CustomEvent<DndEvent<AnyRow>>) {
		rows = e.detail.items;
	}

	function handleDndFinalize(e: CustomEvent<DndEvent<AnyRow>>) {
		rows = e.detail.items;
		sync();
	}
</script>

<div class="space-y-2">
	<div
		class="space-y-2"
		use:dndzone={{ items: rows, flipDurationMs: 150 }}
		onconsider={handleDndConsider}
		onfinalize={handleDndFinalize}
	>
	{#each rows as row, index (row.id)}
		<div class="flex items-center gap-1">
			<span
				class="flex size-6 shrink-0 cursor-grab touch-none items-center justify-center text-muted-foreground active:cursor-grabbing"
				aria-label={m.string_list_drag_handle()}
			>
				<GripVerticalIcon class="size-4" />
			</span>
			<Input
				{placeholder}
				{inputmode}
				bind:value={() => textOf(row), (v) => { setText(row, v); sync(); }}
			/>
			<Button
				type="button"
				variant="ghost"
				size="icon-xs"
				class="shrink-0 text-muted-foreground"
				disabled={index === 0}
				onclick={() => moveUp(index)}
				aria-label={m.string_list_move_up()}
			>
				<ChevronUpIcon />
			</Button>
			<Button
				type="button"
				variant="ghost"
				size="icon-xs"
				class="shrink-0 text-muted-foreground"
				disabled={index === rows.length - 1}
				onclick={() => moveDown(index)}
				aria-label={m.string_list_move_down()}
			>
				<ChevronDownIcon />
			</Button>
			<Button
				type="button"
				variant="ghost"
				size="icon"
				class="text-muted-foreground shrink-0"
				onclick={() => remove(row.id)}
				aria-label={m.string_list_remove()}
			>
				<XIcon />
			</Button>
		</div>
	{/each}
</div>

	<Button type="button" variant="outline" size="sm" onclick={add}>
		<PlusIcon />
		{addLabel}
	</Button>
</div>
