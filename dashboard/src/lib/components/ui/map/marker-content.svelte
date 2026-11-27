<script lang="ts">
	import { Portal } from 'bits-ui';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { useMarkerContext } from './context.js';

	type Props = {
		/** Custom marker content. Defaults to a blue dot if not provided */
		children?: Snippet;
		/** Additional CSS classes for the marker container */
		class?: string;
	};

	let { children, class: className }: Props = $props();

	const { marker } = useMarkerContext();
</script>

<Portal to={marker.getElement()}>
	<div class={cn('relative cursor-pointer', className)}>
		{#if children}
			{@render children()}
		{:else}
			<div class="relative h-4 w-4 rounded-full border-2 border-white bg-blue-500 shadow-lg"></div>
		{/if}
	</div>
</Portal>
