<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children?: Snippet } = $props();
</script>

<!-- Numbered walkthrough: each `###` heading inside becomes a step, auto-numbered by CSS so
     inserting a step in the middle doesn't require renumbering the markdown. -->
<div
	class="ml-4 border-l pl-8 [counter-reset:step] [&>h3]:relative [&>h3]:mt-8 [&>h3]:[counter-increment:step] [&>h3]:before:absolute [&>h3]:before:-left-4 [&>h3]:before:flex [&>h3]:before:size-8 [&>h3]:before:-translate-x-1/2 [&>h3]:before:items-center [&>h3]:before:justify-center [&>h3]:before:rounded-full [&>h3]:before:border [&>h3]:before:bg-muted [&>h3]:before:text-sm [&>h3]:before:font-medium [&>h3]:before:content-[counter(step)]"
>
	{@render children?.()}
</div>
