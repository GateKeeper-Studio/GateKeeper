<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import HashIcon from '@lucide/svelte/icons/hash';
	import FileTextIcon from '@lucide/svelte/icons/file-text';

	import { afterNavigate } from '$app/navigation';
	import docsIndex from 'virtual:docs-search-index';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Command from '$lib/components/ui/command/index.js';

	let open = $state(false);
	let search = $state('');

	const terms = $derived(search.toLowerCase().split(/\s+/).filter(Boolean));

	// Ranked here rather than by the Command's built-in filter: the index holds a few hundred
	// entries with long bodies, and the built-in matcher would score (and mount) every one of
	// them on each keystroke. Capping at 20 keeps typing responsive.
	const results = $derived.by(() => {
		if (terms.length === 0) return [];

		return docsIndex
			.map((entry) => {
				const haystack = `${entry.title} ${entry.heading ?? ''} ${entry.text}`.toLowerCase();

				let score = 0;
				for (const term of terms) {
					// Every term must appear somewhere — an AND search, not an OR.
					if (!haystack.includes(term)) return { entry, score: 0 };
					if (entry.title.toLowerCase().includes(term)) score += 5;
					if (entry.heading?.toLowerCase().includes(term)) score += 3;
					score += 1;
				}

				return { entry, score };
			})
			.filter((hit) => hit.score > 0)
			.sort((a, b) => b.score - a.score)
			.slice(0, 20)
			.map((hit) => hit.entry);
	});

	afterNavigate(() => (open = false));

	$effect(() => {
		if (!open) search = '';
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'k' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			open = !open;
		}
	}
</script>

<svelte:window {onkeydown} />

<Button
	variant="outline"
	size="sm"
	class="w-full justify-start text-muted-foreground md:w-56"
	onclick={() => (open = true)}
>
	<SearchIcon class="mr-2 size-4" />
	<span class="flex-1 text-left">Search docs…</span>
	<kbd class="hidden text-xs md:inline">⌘K</kbd>
</Button>

<Command.Dialog bind:open shouldFilter={false} title="Search documentation">
	<Command.Input placeholder="Search the docs…" bind:value={search} />
	<Command.List>
		<Command.Empty>{search ? 'No results found.' : 'Type to search…'}</Command.Empty>

		{#if results.length > 0}
			<Command.Group heading="Documentation">
				{#each results as result (result.id)}
					<Command.LinkItem href={result.url} value={result.id} onSelect={() => (open = false)}>
						{#if result.heading}
							<HashIcon />
						{:else}
							<FileTextIcon />
						{/if}
						<div class="flex min-w-0 flex-col">
							<span class="truncate">{result.heading ?? result.title}</span>
							<span class="truncate text-xs text-muted-foreground">
								{result.heading ? `${result.title} — ` : ''}{result.text.slice(0, 90)}
							</span>
						</div>
					</Command.LinkItem>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
