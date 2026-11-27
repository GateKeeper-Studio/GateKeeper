<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';
	import { Portal } from 'bits-ui';
	import { Popup, type PopupOptions } from 'maplibre-gl';
	import { untrack, type Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { useMap } from './context.js';

	type Props = Omit<PopupOptions, 'className' | 'closeButton'> & {
		/** Longitude coordinate for popup position */
		longitude: number;
		/** Latitude coordinate for popup position */
		latitude: number;
		/** Called when the popup is closed */
		onClose?: () => void;
		/** Popup content */
		children: Snippet;
		/** Additional CSS classes for the popup container */
		class?: string;
		/** Show a close button in the popup (default: false) */
		closeButton?: boolean;
	};

	let {
		longitude,
		latitude,
		onClose,
		children,
		class: className,
		closeButton = false,
		...popupOptions
	}: Props = $props();

	const mapContext = useMap();
	const container = document.createElement('div');

	const popup = untrack(() =>
		new Popup({ offset: 16, ...popupOptions, closeButton: false })
			.setMaxWidth(popupOptions.maxWidth ?? 'none')
			.setLngLat([longitude, latitude])
			.setDOMContent(container)
	);

	$effect(() => {
		const map = mapContext.map;
		if (!map) return;

		// MapLibre fires `close` for every removal: the close button, closeOnClick,
		// closeOnMove. The handler is detached before teardown removes the popup.
		const handleClose = () => onClose?.();
		popup.on('close', handleClose);
		popup.addTo(map);

		return () => {
			popup.off('close', handleClose);
			if (popup.isOpen()) {
				popup.remove();
			}
		};
	});

	$effect(() => {
		const current = popup.getLngLat();
		if (current.lng !== longitude || current.lat !== latitude) {
			popup.setLngLat([longitude, latitude]);
		}
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
			<!-- remove() fires `close`, which calls onClose. -->
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
