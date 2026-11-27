<script lang="ts">
	import { Marker, type MarkerOptions } from 'maplibre-gl';
	import { untrack, type Snippet } from 'svelte';
	import { setMarkerContext, useMap } from './context.js';

	type LngLatPosition = { lng: number; lat: number };

	type Props = Omit<MarkerOptions, 'element'> & {
		/** Longitude coordinate for marker position */
		longitude: number;
		/** Latitude coordinate for marker position */
		latitude: number;
		/** Marker subcomponents (MarkerContent, MarkerPopup, MarkerTooltip, MarkerLabel) */
		children: Snippet;
		/** Called when the marker is clicked */
		onclick?: (event: MouseEvent) => void;
		/** Called when the mouse enters the marker */
		onmouseenter?: (event: MouseEvent) => void;
		/** Called when the mouse leaves the marker */
		onmouseleave?: (event: MouseEvent) => void;
		/** Called when a marker drag starts (requires `draggable`) */
		onDragStart?: (lngLat: LngLatPosition) => void;
		/** Called during a marker drag (requires `draggable`) */
		onDrag?: (lngLat: LngLatPosition) => void;
		/** Called when a marker drag ends (requires `draggable`) */
		onDragEnd?: (lngLat: LngLatPosition) => void;
	};

	let {
		longitude,
		latitude,
		children,
		onclick,
		onmouseenter,
		onmouseleave,
		onDragStart,
		onDrag,
		onDragEnd,
		draggable = false,
		...markerOptions
	}: Props = $props();

	const mapContext = useMap();

	// Built once from the initial props; the effects below apply later changes to it.
	const marker = untrack(() =>
		new Marker({
			...markerOptions,
			element: document.createElement('div'),
			draggable
		}).setLngLat([longitude, latitude])
	);

	// The handlers read the callback props when they fire, so they are always current.
	const element = marker.getElement();
	element.addEventListener('click', (event) => onclick?.(event));
	element.addEventListener('mouseenter', (event) => onmouseenter?.(event));
	element.addEventListener('mouseleave', (event) => onmouseleave?.(event));

	const position = (): LngLatPosition => {
		const { lng, lat } = marker.getLngLat();
		return { lng, lat };
	};
	marker.on('dragstart', () => onDragStart?.(position()));
	marker.on('drag', () => onDrag?.(position()));
	marker.on('dragend', () => onDragEnd?.(position()));

	setMarkerContext({
		marker,
		get map() {
			return mapContext.map;
		}
	});

	$effect(() => {
		const map = mapContext.map;
		if (!map) return;

		marker.addTo(map);
		return () => {
			marker.remove();
		};
	});

	$effect(() => {
		const current = marker.getLngLat();
		if (current.lng !== longitude || current.lat !== latitude) {
			marker.setLngLat([longitude, latitude]);
		}
	});

	$effect(() => {
		if (marker.isDraggable() !== draggable) {
			marker.setDraggable(draggable);
		}
	});

	$effect(() => {
		const offset = markerOptions.offset ?? [0, 0];
		const [x, y] = Array.isArray(offset) ? offset : [offset.x, offset.y];
		const current = marker.getOffset();
		if (current.x !== x || current.y !== y) {
			marker.setOffset(offset);
		}
	});

	$effect(() => {
		const rotation = markerOptions.rotation ?? 0;
		if (marker.getRotation() !== rotation) {
			marker.setRotation(rotation);
		}
	});

	$effect(() => {
		const alignment = markerOptions.rotationAlignment ?? 'auto';
		if (marker.getRotationAlignment() !== alignment) {
			marker.setRotationAlignment(alignment);
		}
	});

	$effect(() => {
		const alignment = markerOptions.pitchAlignment ?? 'auto';
		if (marker.getPitchAlignment() !== alignment) {
			marker.setPitchAlignment(alignment);
		}
	});
</script>

{@render children()}
