<script lang="ts" generics="P extends GeoJsonProperties = GeoJsonProperties">
	import type { Feature, FeatureCollection, GeoJsonProperties, Point } from 'geojson';
	import type { ExpressionSpecification, GeoJSONSource, MapLayerMouseEvent } from 'maplibre-gl';
	import { untrack } from 'svelte';
	import { useMap } from './context.js';

	type Props = {
		/** GeoJSON FeatureCollection data or URL to fetch GeoJSON from */
		data: string | FeatureCollection<Point, P>;
		/** Maximum zoom level to cluster points on (default: 14) */
		clusterMaxZoom?: number;
		/** Radius of each cluster when clustering points in pixels (default: 50) */
		clusterRadius?: number;
		/** Colors for cluster circles: [small, medium, large] based on point count (default: ["#51bbd6", "#f1f075", "#f28cb1"]) */
		clusterColors?: [string, string, string];
		/** Point count thresholds for color/size steps: [medium, large] (default: [100, 750]) */
		clusterThresholds?: [number, number];
		/** Color for unclustered individual points (default: "#3b82f6") */
		pointColor?: string;
		/** Called when an unclustered point is clicked */
		onPointClick?: (feature: Feature<Point, P>, coordinates: [number, number]) => void;
		/** Called when a cluster is clicked. If not provided, zooms into the cluster */
		onClusterClick?: (clusterId: number, coordinates: [number, number], pointCount: number) => void;
	};

	let {
		data,
		clusterMaxZoom = 14,
		clusterRadius = 50,
		clusterColors = ['#51bbd6', '#f1f075', '#f28cb1'],
		clusterThresholds = [100, 750],
		pointColor = '#3b82f6',
		onPointClick,
		onClusterClick
	}: Props = $props();

	const mapContext = useMap();
	const id = $props.id();
	const sourceId = `cluster-source-${id}`;
	const clusterLayerId = `clusters-${id}`;
	const clusterCountLayerId = `cluster-count-${id}`;
	const unclusteredLayerId = `unclustered-point-${id}`;

	function circleColor(
		colors: [string, string, string],
		thresholds: [number, number]
	): ExpressionSpecification {
		return [
			'step',
			['get', 'point_count'],
			colors[0],
			thresholds[0],
			colors[1],
			thresholds[1],
			colors[2]
		];
	}

	function circleRadius(thresholds: [number, number]): ExpressionSpecification {
		return ['step', ['get', 'point_count'], 20, thresholds[0], 30, thresholds[1], 40];
	}

	// Add the source and layers once the style is ready. They are re-added after a
	// theme change, since setStyle drops them; data and style changes go through the
	// effects below.
	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map) return;

		untrack(() => {
			map.addSource(sourceId, {
				type: 'geojson',
				// Snapshot: `$state` data is a proxy, which MapLibre cannot send to its worker.
				data: $state.snapshot(data) as string | FeatureCollection,
				cluster: true,
				clusterMaxZoom,
				clusterRadius
			});

			map.addLayer({
				id: clusterLayerId,
				type: 'circle',
				source: sourceId,
				filter: ['has', 'point_count'],
				paint: {
					'circle-color': circleColor(clusterColors, clusterThresholds),
					'circle-radius': circleRadius(clusterThresholds)
				}
			});

			map.addLayer({
				id: clusterCountLayerId,
				type: 'symbol',
				source: sourceId,
				filter: ['has', 'point_count'],
				layout: {
					'text-field': '{point_count_abbreviated}',
					'text-size': 12
				},
				paint: {
					'text-color': '#fff'
				}
			});

			map.addLayer({
				id: unclusteredLayerId,
				type: 'circle',
				source: sourceId,
				filter: ['!', ['has', 'point_count']],
				paint: {
					'circle-color': pointColor,
					'circle-radius': 6
				}
			});
		});

		return () => {
			try {
				if (map.getLayer(clusterCountLayerId)) map.removeLayer(clusterCountLayerId);
				if (map.getLayer(unclusteredLayerId)) map.removeLayer(unclusteredLayerId);
				if (map.getLayer(clusterLayerId)) map.removeLayer(clusterLayerId);
				if (map.getSource(sourceId)) map.removeSource(sourceId);
			} catch {
				// The map may already be gone.
			}
		};
	});

	// Update the source data when the data prop changes (only for non-URL data).
	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || typeof data === 'string') return;

		const snapshot = $state.snapshot(data) as FeatureCollection;
		void map.getSource<GeoJSONSource>(sourceId)?.setData(snapshot);
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || !map.getLayer(clusterLayerId)) return;

		const thresholds = $state.snapshot(clusterThresholds);
		map.setPaintProperty(
			clusterLayerId,
			'circle-color',
			circleColor($state.snapshot(clusterColors), thresholds)
		);
		map.setPaintProperty(clusterLayerId, 'circle-radius', circleRadius(thresholds));
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map || !map.getLayer(unclusteredLayerId)) return;

		map.setPaintProperty(unclusteredLayerId, 'circle-color', pointColor);
	});

	$effect(() => {
		const map = mapContext.map;
		if (!mapContext.isLoaded || !map) return;

		// Zoom into a clicked cluster, unless the caller handles the click.
		const handleClusterClick = async (event: MapLayerMouseEvent) => {
			const [feature] = map.queryRenderedFeatures(event.point, {
				layers: [clusterLayerId]
			});
			if (!feature) return;

			const clusterId = feature.properties?.cluster_id as number;
			const pointCount = feature.properties?.point_count as number;
			const coordinates = (feature.geometry as Point).coordinates as [number, number];

			if (onClusterClick) {
				onClusterClick(clusterId, coordinates, pointCount);
			} else {
				const source = map.getSource<GeoJSONSource>(sourceId);
				if (!source) return;
				const zoom = await source.getClusterExpansionZoom(clusterId);
				map.easeTo({ center: coordinates, zoom });
			}
		};

		const handlePointClick = (event: MapLayerMouseEvent) => {
			const feature = event.features?.[0];
			if (!onPointClick || !feature) return;

			const coordinates = (feature.geometry as Point).coordinates.slice() as [number, number];

			// Pick the copy of the point nearest the click when the world wraps.
			while (Math.abs(event.lngLat.lng - coordinates[0]) > 180) {
				coordinates[0] += event.lngLat.lng > coordinates[0] ? 360 : -360;
			}

			onPointClick(feature as unknown as Feature<Point, P>, coordinates);
		};

		const canvas = map.getCanvas();
		const subscriptions = [
			map.on('click', clusterLayerId, handleClusterClick),
			map.on('click', unclusteredLayerId, handlePointClick),
			map.on('mouseenter', clusterLayerId, () => {
				canvas.style.cursor = 'pointer';
			}),
			map.on('mouseleave', clusterLayerId, () => {
				canvas.style.cursor = '';
			}),
			map.on('mouseenter', unclusteredLayerId, () => {
				// Read when the pointer enters, so it follows the current prop.
				if (onPointClick) canvas.style.cursor = 'pointer';
			}),
			map.on('mouseleave', unclusteredLayerId, () => {
				canvas.style.cursor = '';
			})
		];

		return () => {
			for (const subscription of subscriptions) subscription.unsubscribe();
		};
	});
</script>
