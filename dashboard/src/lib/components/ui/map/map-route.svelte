<script lang="ts">
	import type { Feature, LineString } from 'geojson';
	import type { GeoJSONSource } from 'maplibre-gl';
	import { untrack } from 'svelte';
	import { useMap } from './context.js';

	type Props = {
		/** Optional unique identifier for the route layer */
		id?: string;
		/** Array of [longitude, latitude] coordinate pairs defining the route */
		coordinates: [number, number][];
		/** Line color as CSS color value (default: "#4285F4") */
		color?: string;
		/** Line width in pixels (default: 3) */
		width?: number;
		/** Line opacity from 0 to 1 (default: 0.8) */
		opacity?: number;
		/** Dash pattern [dash length, gap length] for dashed lines */
		dashArray?: [number, number];
		/** Called when the route line is clicked */
		onclick?: () => void;
		/** Called when the mouse enters the route line */
		onmouseenter?: () => void;
		/** Called when the mouse leaves the route line */
		onmouseleave?: () => void;
		/** Whether the route is interactive - shows pointer cursor on hover (default: true) */
		interactive?: boolean;
	};

	let {
		id: propId,
		coordinates,
		color = '#4285F4',
		width = 3,
		opacity = 0.8,
		dashArray,
		onclick,
		onmouseenter,
		onmouseleave,
		interactive = true
	}: Props = $props();

	const mapContext = useMap();
	const autoId = $props.id();
	const id = $derived(propId ?? autoId);
	const sourceId = $derived(`route-source-${id}`);
	const layerId = $derived(`route-layer-${id}`);

	function line(points: [number, number][]): Feature<LineString> {
		return {
			type: 'Feature',
			properties: {},
			geometry: { type: 'LineString', coordinates: points }
		};
	}

	// Add the source and layer once the style is ready. They are re-added after a
	// theme change, since setStyle drops them; paint changes go through the effect below.
	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map) return;

		const source = sourceId;
		const layer = layerId;
		untrack(() => {
			map.addSource(source, { type: 'geojson', data: line([]) });
			map.addLayer({
				id: layer,
				type: 'line',
				source,
				layout: { 'line-join': 'round', 'line-cap': 'round' },
				paint: {
					'line-color': color,
					'line-width': width,
					'line-opacity': opacity,
					...(dashArray && { 'line-dasharray': $state.snapshot(dashArray) })
				}
			});
		});

		return () => {
			try {
				if (map.getLayer(layer)) map.removeLayer(layer);
				if (map.getSource(source)) map.removeSource(source);
			} catch {
				// The map may already be gone.
			}
		};
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || coordinates.length < 2) return;

		// Snapshot: a `$state` array is a proxy, which MapLibre cannot send to its worker.
		void map.getSource<GeoJSONSource>(sourceId)?.setData(line($state.snapshot(coordinates)));
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || !map.getLayer(layerId)) return;

		map.setPaintProperty(layerId, 'line-color', color);
		map.setPaintProperty(layerId, 'line-width', width);
		map.setPaintProperty(layerId, 'line-opacity', opacity);
		if (dashArray) {
			map.setPaintProperty(layerId, 'line-dasharray', $state.snapshot(dashArray));
		}
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || !interactive) return;

		// The handlers read the callback props when they fire, so they are always current.
		const subscriptions = [
			map.on('click', layerId, () => onclick?.()),
			map.on('mouseenter', layerId, () => {
				map.getCanvas().style.cursor = 'pointer';
				onmouseenter?.();
			}),
			map.on('mouseleave', layerId, () => {
				map.getCanvas().style.cursor = '';
				onmouseleave?.();
			})
		];

		return () => {
			for (const subscription of subscriptions) subscription.unsubscribe();
		};
	});
</script>
