<script lang="ts" module>
	import { setWorkerUrl } from 'maplibre-gl';
	import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?url';

	// MapLibre computes its worker's address from its own module URL at run time,
	// which Vite cannot see, so a build would ship no worker and every map would fail
	// to load. Import the worker as an asset and give MapLibre that address. It is a
	// same-origin module script: the server's CSP allows it (`presentation/ui.rs`).
	setWorkerUrl(workerUrl);
</script>

<script lang="ts">
	import 'maplibre-gl/dist/maplibre-gl.css';
	import {
		Map as MapLibreMap,
		type MapOptions,
		type ProjectionSpecification,
		type StyleSpecification
	} from 'maplibre-gl';
	import { mode, systemPrefersMode } from 'mode-watcher';
	import { onMount, type Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { setMapContext } from './context.js';

	type MapStyleOption = string | StyleSpecification;

	type Props = Omit<MapOptions, 'container' | 'style'> & {
		children?: Snippet;
		/**
		 * Theme for the map. If not provided, follows the app's light/dark mode
		 * (mode-watcher), falling back to the system preference.
		 */
		theme?: 'light' | 'dark';
		/** Custom map styles for light and dark themes. Overrides the default Carto styles. */
		styles?: {
			light?: MapStyleOption;
			dark?: MapStyleOption;
		};
		/** Map projection type. Use `{ type: 'globe' }` for 3D globe view. */
		projection?: ProjectionSpecification;
		/** Extra classes for the map container. */
		class?: string;
		/**
		 * The MapLibre instance (`bind:map`); null until the map is mounted. A class
		 * instance, so `$state` never wraps it in a proxy.
		 */
		map?: MapLibreMap | null;
	};

	const defaultStyles = {
		dark: 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json',
		light: 'https://basemaps.cartocdn.com/gl/positron-gl-style/style.json'
	};

	let {
		children,
		theme,
		styles,
		projection,
		class: className,
		map = $bindable(null),
		...options
	}: Props = $props();

	let container: HTMLDivElement;
	let isLoaded = $state(false);
	let isStyleLoaded = $state(false);
	let currentStyle: MapStyleOption | null = null;
	let styleTimeout: ReturnType<typeof setTimeout> | null = null;

	const resolvedTheme = $derived(theme ?? mode.current ?? systemPrefersMode.current ?? 'light');
	const activeStyle = $derived<MapStyleOption>(
		resolvedTheme === 'dark'
			? (styles?.dark ?? defaultStyles.dark)
			: (styles?.light ?? defaultStyles.light)
	);

	setMapContext({
		get map() {
			return map;
		},
		get isLoaded() {
			return isLoaded && isStyleLoaded;
		}
	});

	// A style object passed from `$state` is a proxy, which MapLibre cannot clone to its
	// workers. Snapshotting through `object` keeps TypeScript from expanding the deep
	// StyleSpecification type.
	function plainStyle(style: MapStyleOption): MapStyleOption {
		return typeof style === 'string'
			? style
			: ($state.snapshot(style as object) as StyleSpecification);
	}

	function clearStyleTimeout() {
		if (styleTimeout) {
			clearTimeout(styleTimeout);
			styleTimeout = null;
		}
	}

	onMount(() => {
		currentStyle = activeStyle;

		const created = new MapLibreMap({
			container,
			style: plainStyle(activeStyle),
			renderWorldCopies: false,
			attributionControl: {
				compact: true
			},
			...options
		});

		const handleStyleData = () => {
			clearStyleTimeout();
			// Delay so the style is fully processed before children add sources and
			// layers. This avoids racing the style load; otherwise every layer would
			// have to be re-added after each setStyle.
			styleTimeout = setTimeout(() => {
				isStyleLoaded = true;
				if (projection) {
					created.setProjection(projection);
				}
			}, 100);
		};
		const handleLoad = () => {
			isLoaded = true;
		};

		created.on('load', handleLoad);
		created.on('styledata', handleStyleData);
		map = created;

		return () => {
			clearStyleTimeout();
			created.off('load', handleLoad);
			created.off('styledata', handleStyleData);
			created.remove();
			isLoaded = false;
			isStyleLoaded = false;
			map = null;
		};
	});

	// Swap the style when the theme (or the styles prop) changes.
	$effect(() => {
		const nextStyle = activeStyle;
		if (!map || currentStyle === nextStyle) return;

		clearStyleTimeout();
		currentStyle = nextStyle;
		isStyleLoaded = false;
		map.setStyle(plainStyle(nextStyle), { diff: true });
	});
</script>

<div bind:this={container} class={cn('relative h-full w-full', className)}>
	{#if !isLoaded}
		<div class="absolute inset-0 flex items-center justify-center">
			<div class="flex gap-1">
				<span class="size-1.5 animate-pulse rounded-full bg-muted-foreground/60"></span>
				<span
					class="size-1.5 animate-pulse rounded-full bg-muted-foreground/60 [animation-delay:150ms]"
				></span>
				<span
					class="size-1.5 animate-pulse rounded-full bg-muted-foreground/60 [animation-delay:300ms]"
				></span>
			</div>
		</div>
	{/if}
	<!-- Children (markers, layers, controls) need the instance, so they wait for it. -->
	{#if map}
		{@render children?.()}
	{/if}
</div>

<style>
	/* The popups and tooltips draw their own card; drop MapLibre's frame and tip. */
	:global(.maplibregl-popup-content) {
		padding: 0;
		background: transparent;
		box-shadow: none;
	}
	:global(.maplibregl-popup-tip) {
		display: none;
	}
</style>
