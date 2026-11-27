import type { Map as MapLibreMap } from 'maplibre-gl';
import Root from './map.svelte';
import Marker from './map-marker.svelte';
import MarkerContent from './marker-content.svelte';
import MarkerPopup from './marker-popup.svelte';
import MarkerTooltip from './marker-tooltip.svelte';
import MarkerLabel from './marker-label.svelte';
import Popup from './map-popup.svelte';
import Controls from './map-controls.svelte';
import Route from './map-route.svelte';
import ClusterLayer from './map-cluster-layer.svelte';

/** The MapLibre instance a `<Map bind:map>` exposes. */
type MapRef = MapLibreMap;

export { useMap, useMarkerContext, type MapContext, type MarkerContext } from './context.js';

export {
	Root,
	Marker,
	Popup,
	Controls,
	Route,
	ClusterLayer,
	//
	Root as Map,
	Marker as MapMarker,
	MarkerContent,
	MarkerPopup,
	MarkerTooltip,
	MarkerLabel,
	Popup as MapPopup,
	Controls as MapControls,
	Route as MapRoute,
	ClusterLayer as MapClusterLayer,
	type MapRef
};
