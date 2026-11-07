<script lang="ts">
	import type { DocsHeading } from '$lib/docs/types';

	let { headings = [] }: { headings?: DocsHeading[] } = $props();

	let active = $state('');

	// The heading list itself is built at compile time, so the TOC is server-rendered. This
	// effect only adds scrollspy highlighting on top of it.
	$effect(() => {
		const elements = headings
			.map((heading) => document.getElementById(heading.id))
			.filter((el): el is HTMLElement => !!el);

		if (elements.length === 0) return;

		const observer = new IntersectionObserver(
			(entries) => {
				const visible = entries.find((entry) => entry.isIntersecting);
				if (visible) active = visible.target.id;
			},
			// Trip the moment a heading clears the sticky header, not when it's centred.
			{ rootMargin: '-96px 0px -70% 0px' }
		);

		elements.forEach((el) => observer.observe(el));
		return () => observer.disconnect();
	});
</script>

{#if headings.length > 0}
	<aside class="sticky top-24 hidden h-fit w-56 shrink-0 xl:block">
		<p class="mb-3 text-sm font-semibold">On this page</p>
		<nav class="flex flex-col gap-2 border-l text-sm">
			{#each headings as heading (heading.id)}
				<a
					href="#{heading.id}"
					class="-ml-px border-l-2 transition-colors hover:text-foreground {heading.depth === 3
						? 'pl-6'
						: 'pl-4'} {active === heading.id
						? 'border-primary font-medium text-foreground'
						: 'border-transparent text-muted-foreground'}"
				>
					{heading.text}
				</a>
			{/each}
		</nav>
	</aside>
{/if}
