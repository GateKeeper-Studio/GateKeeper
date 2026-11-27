import { getContext, setContext } from 'svelte';
import type { Map as MapLibreMap, Marker } from 'maplibre-gl';

export type MapContext = {
	/** The MapLibre instance. Children only render once it exists. */
	readonly map: MapLibreMap | null;
	/** True once the map and its current style are ready for sources and layers. */
	readonly isLoaded: boolean;
};

export type MarkerContext = {
	readonly marker: Marker;
	readonly map: MapLibreMap | null;
};

const MAP_KEY = Symbol('map');
const MARKER_KEY = Symbol('map-marker');

export function setMapContext(context: MapContext): void {
	setContext(MAP_KEY, context);
}

/**
 * Reads the enclosing `<Map>`. Call it during component initialisation; the returned
 * object is reactive, so read `map` / `isLoaded` inside effects or the template.
 */
export function useMap(): MapContext {
	const context = getContext<MapContext | undefined>(MAP_KEY);
	if (!context) {
		throw new Error('useMap must be used within a Map component');
	}
	return context;
}

export function setMarkerContext(context: MarkerContext): void {
	setContext(MARKER_KEY, context);
}

export function useMarkerContext(): MarkerContext {
	const context = getContext<MarkerContext | undefined>(MARKER_KEY);
	if (!context) {
		throw new Error('Marker components must be used within MapMarker');
	}
	return context;
}
