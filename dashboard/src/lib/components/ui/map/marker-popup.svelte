<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
	import { Portal } from 'bits-ui';
	import { Popup, type PopupOptions } from 'maplibre-gl';
	import { untrack, type Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { useMarkerContext } from './context.js';

	type Props = Omit<PopupOptions, 'className' | 'closeButton'> & {
		/** Popup content */
		children: Snippet;
		/** Additional CSS classes for the popup container */
		class?: string;
		/** Show a close button in the popup (default: false) */
		closeButton?: boolean;
	};

	let { children, class: className, closeButton = false, ...popupOptions }: Props = $props();

	const markerContext = useMarkerContext();
	const container = document.createElement('div');

	// MapLibre's own close button is off; the styled one below replaces it.
	const popup = untrack(() =>
		new Popup({ offset: 16, ...popupOptions, closeButton: false })
			.setMaxWidth(popupOptions.maxWidth ?? 'none')
			.setDOMContent(container)
	);

	// Clicking the marker toggles the popup.
	$effect(() => {
		if (!markerContext.map) return;

		const { marker } = markerContext;
		marker.setPopup(popup);
		return () => {
			marker.setPopup(null);
		};
	});

	$effect(() => {
		popup.setOffset(popupOptions.offset ?? 16);
	});

	$effect(() => {
		popup.setMaxWidth(popupOptions.maxWidth ?? 'none');
	});
</script>

<Portal to={container}>
	<div
		class={cn(
			'relative animate-in rounded-md border bg-popover p-3 text-popover-foreground shadow-md fade-in-0 zoom-in-95',
			className
		)}
	>
		{#if closeButton}
			<button
				type="button"
				onclick={() => popup.remove()}
				class="absolute top-1 right-1 z-10 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:outline-none"
				aria-label="Close popup"
			>
				<XIcon class="h-4 w-4" />
				<span class="sr-only">Close</span>
			</button>
		{/if}
		{@render children()}
	</div>
</Portal>
