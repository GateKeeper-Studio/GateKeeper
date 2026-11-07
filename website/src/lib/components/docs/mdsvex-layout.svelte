<script module>
	// These named exports are the whole reason mdsvex adds `import * as Components from '<this
	// file>'` to every compiled page — that import is what puts Callout/Mermaid/Steps in scope
	// inside markdown, and what lets the Shiki highlighter emit a bare <Components.Mermaid> tag.
	//
	// Do NOT export anything named after an HTML tag (Pre, Table, Img): mdsvex rewrites matching
	// tag names on markdown-generated elements, so it would swallow every real <table>.
	export { default as Callout } from './callout.svelte';
	export { default as Mermaid } from './mermaid.svelte';
	export { default as Steps } from './steps.svelte';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	import DocsToc from './docs-toc.svelte';
	import { copyCode } from '$lib/docs/copy-code';
	import type { DocsHeading } from '$lib/docs/types';

	// mdsvex spreads the page's frontmatter onto this layout, and `docs-tooling/rehype-headings.js`
	// injected `headings` into that frontmatter — so the TOC arrives already populated, no DOM scan.
	let {
		title,
		description,
		headings = [],
		children
	}: {
		title?: string;
		description?: string;
		headings?: DocsHeading[];
		children?: Snippet;
	} = $props();
</script>

<svelte:head>
	<title>{title ? `${title} - Docs` : 'Docs'} - GateKeeper</title>
	{#if description}
		<meta name="description" content={description} />
	{/if}
</svelte:head>

<div class="flex w-full gap-10">
	<article class="max-w-3xl min-w-0 flex-1">
		{#if title}
			<h1 class="mb-2 text-3xl font-bold tracking-tight">{title}</h1>
			{#if description}
				<p class="mb-8 text-lg text-muted-foreground">{description}</p>
			{/if}
		{/if}

		<!-- `prose-code:before/after:content-none` is not optional: @tailwindcss/typography
		     renders a literal ` on each side of every inline <code>, so `BLOCKED_IP` would
		     come out as `` `BLOCKED_IP` ``. Removing them means we also have to style inline
		     code ourselves, which is what the rest of the prose-code:* utilities do. -->
		<div
			class="prose max-w-none prose-neutral dark:prose-invert prose-headings:scroll-mt-24 prose-a:text-primary prose-code:rounded prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 prose-code:text-[0.85em] prose-code:font-normal prose-code:before:content-none prose-code:after:content-none"
			use:copyCode
		>
			{@render children?.()}
		</div>
	</article>

	<DocsToc {headings} />
</div>
