<script lang="ts">
	import { mode } from 'mode-watcher';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';

	// Base64, set by the Shiki highlighter in mdsvex.config.js — see the note there on why the
	// diagram source can't ride in the attribute raw.
	let { encoded }: { encoded: string } = $props();

	const diagram = $derived(
		new TextDecoder().decode(Uint8Array.from(atob(encoded), (c) => c.charCodeAt(0)))
	);

	let svg = $state('');
	let failed = $state(false);
	let renderCount = 0;

	// $effect never runs on the server, so mermaid — which touches `document` on import and is
	// ~1MB with its d3 dependencies — is only ever loaded in the browser, and never enters the
	// prerender path. Re-runs when the theme flips, since mermaid's theme is global config that
	// has to be re-applied before each render.
	$effect(() => {
		const source = diagram;
		const dark = mode.current === 'dark';
		let cancelled = false;

		(async () => {
			try {
				const { default: mermaid } = await import('mermaid');

				mermaid.initialize({
					startOnLoad: false,
					securityLevel: 'strict',
					theme: dark ? 'dark' : 'default',
					// A concrete font stack, NOT `inherit`: mermaid measures every label to lay the
					// diagram out, and it cannot measure against a font-family it can't resolve —
					// it ends up computing a ~300px width and scaling the whole graph down to an
					// illegible thumbnail.
					fontFamily: 'Inter Variable, Inter, ui-sans-serif, system-ui, sans-serif'
				});

				// Ids must be unique per render — mermaid keys internal state off them.
				const id = `mermaid-${renderCount++}-${Math.random().toString(36).slice(2, 8)}`;
				const { svg: rendered } = await mermaid.render(id, source);

				if (!cancelled) {
					// Mermaid bakes `max-width: <computed>px` into the SVG. Dropping it lets the
					// diagram scale to the column via the CSS below, instead of being pinned to
					// whatever width it happened to measure.
					svg = rendered.replace(/max-width:\s*[\d.]+px;?/g, '');
					failed = false;
				}
			} catch (error) {
				if (!cancelled) {
					failed = true;
					console.error('[mermaid] failed to render diagram', error);
				}
			}
		})();

		return () => {
			cancelled = true;
		};
	});
</script>

<!-- min-h keeps the prerendered layout from jumping when the diagram lands. -->
<figure
	class="not-prose my-6 flex min-h-40 items-center justify-center overflow-x-auto rounded-lg border bg-card p-4"
>
	{#if svg}
		<!-- Diagrams are authored by us, and mermaid sanitizes its own output at securityLevel 'strict'. -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		<div class="w-full [&_svg]:h-auto [&_svg]:w-full [&_svg]:max-w-full">{@html svg}</div>
	{:else if failed}
		<p class="text-sm text-destructive">Failed to render diagram.</p>
	{:else}
		<Skeleton class="h-40 w-full" />
	{/if}
</figure>
