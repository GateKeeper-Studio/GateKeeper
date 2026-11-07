<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { DocsNavSection } from '$lib/docs/types';

	let {
		sections,
		current,
		onnavigate
	}: {
		sections: DocsNavSection[];
		current: string;
		onnavigate?: () => void;
	} = $props();
</script>

<nav class="flex flex-col gap-6 text-sm">
	{#each sections as section (section.title)}
		<div class="flex flex-col gap-1">
			<p class="mb-1 font-semibold">{section.title}</p>

			{#each section.items as item (item.href)}
				<a
					href={item.href}
					onclick={onnavigate}
					class={cn(
						'-ml-px border-l-2 py-1 pl-4 transition-colors',
						current === item.href
							? 'border-primary font-medium text-foreground'
							: 'border-transparent text-muted-foreground hover:border-border hover:text-foreground'
					)}
				>
					{item.title}
				</a>
			{/each}
		</div>
	{/each}
</nav>
