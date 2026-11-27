<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	type Props = {
		/** Label text content */
		children: Snippet;
		/** Additional CSS classes for the label */
		class?: string;
		/** Position of the label relative to the marker (default: "top") */
		position?: 'top' | 'bottom';
	};

	let { children, class: className, position = 'top' }: Props = $props();

	const positionClasses = {
		top: 'bottom-full mb-1',
		bottom: 'top-full mt-1'
	};
</script>

<div
	class={cn(
		'absolute left-1/2 -translate-x-1/2 whitespace-nowrap',
		'text-[10px] font-medium text-foreground',
		positionClasses[position],
		className
	)}
>
	{@render children()}
</div>
