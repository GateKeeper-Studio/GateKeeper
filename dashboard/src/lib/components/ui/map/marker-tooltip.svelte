<script lang="ts">
	import { Portal } from 'bits-ui';
	import { Popup, type PopupOptions } from 'maplibre-gl';
	import { untrack, type Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { useMarkerContext } from './context.js';

	type Props = Omit<PopupOptions, 'className' | 'closeButton' | 'closeOnClick'> & {
		/** Tooltip content */
		children: Snippet;
		/** Additional CSS classes for the tooltip container */
		class?: string;
	};

	let { children, class: className, ...popupOptions }: Props = $props();

	const markerContext = useMarkerContext();
	const container = document.createElement('div');

	const tooltip = untrack(() =>
		new Popup({
			offset: 16,
			...popupOptions,
			closeOnClick: true,
			closeButton: false
		})
			.setMaxWidth(popupOptions.maxWidth ?? 'none')
			.setDOMContent(container)
	);

	// Shown while the pointer is over the marker.
	$effect(() => {
		const map = markerContext.map;
		if (!map) return;

		const { marker } = markerContext;
		const element = marker.getElement();
		const show = () => {
			tooltip.setLngLat(marker.getLngLat()).addTo(map);
		};
		const hide = () => {
			tooltip.remove();
		};

		element.addEventListener('mouseenter', show);
		element.addEventListener('mouseleave', hide);
		return () => {
			element.removeEventListener('mouseenter', show);
			element.removeEventListener('mouseleave', hide);
			tooltip.remove();
		};
	});

	$effect(() => {
		tooltip.setOffset(popupOptions.offset ?? 16);
	});

	$effect(() => {
		tooltip.setMaxWidth(popupOptions.maxWidth ?? 'none');
	});
</script>

<Portal to={container}>
	<div
		class={cn(
			'animate-in rounded-md bg-foreground px-2 py-1 text-xs text-background shadow-md fade-in-0 zoom-in-95',
			className
		)}
	>
		{@render children()}
	</div>
</Portal>
