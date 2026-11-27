<script lang="ts">
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import LocateIcon from '@lucide/svelte/icons/locate';
	import MaximizeIcon from '@lucide/svelte/icons/maximize';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import type { Attachment } from 'svelte/attachments';
	import { cn } from '$lib/utils.js';
	import { useMap } from './context.js';

	type Position = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

	type Props = {
		/** Position of the controls on the map (default: "bottom-right") */
		position?: Position;
		/** Show zoom in/out buttons (default: true) */
		showZoom?: boolean;
		/** Show compass button to reset bearing (default: false) */
		showCompass?: boolean;
		/** Show locate button to find user's location (default: false) */
		showLocate?: boolean;
		/** Show fullscreen toggle button (default: false) */
		showFullscreen?: boolean;
		/** Additional CSS classes for the controls container */
		class?: string;
		/** Called with the user's coordinates once located */
		onLocate?: (coords: { longitude: number; latitude: number }) => void;
	};

	let {
		position = 'bottom-right',
		showZoom = true,
		showCompass = false,
		showLocate = false,
		showFullscreen = false,
		class: className,
		onLocate
	}: Props = $props();

	const mapContext = useMap();
	let waitingForLocation = $state(false);

	const positionClasses: Record<Position, string> = {
		'top-left': 'top-2 left-2',
		'top-right': 'top-2 right-2',
		'bottom-left': 'bottom-2 left-2',
		'bottom-right': 'bottom-10 right-2'
	};

	const groupClass =
		'flex flex-col overflow-hidden rounded-md border border-border bg-background shadow-sm [&>button:not(:last-child)]:border-b [&>button:not(:last-child)]:border-border';
	const buttonClass =
		'flex size-8 items-center justify-center transition-colors hover:bg-accent dark:hover:bg-accent/40';

	function zoomBy(delta: number) {
		const map = mapContext.map;
		map?.zoomTo(map.getZoom() + delta, { duration: 300 });
	}

	function resetBearing() {
		mapContext.map?.resetNorthPitch({ duration: 300 });
	}

	function locate() {
		if (!('geolocation' in navigator)) return;

		waitingForLocation = true;
		navigator.geolocation.getCurrentPosition(
			(pos) => {
				const coords = {
					longitude: pos.coords.longitude,
					latitude: pos.coords.latitude
				};
				mapContext.map?.flyTo({
					center: [coords.longitude, coords.latitude],
					zoom: 14,
					duration: 1500
				});
				onLocate?.(coords);
				waitingForLocation = false;
			},
			(error) => {
				console.error('Error getting location:', error);
				waitingForLocation = false;
			}
		);
	}

	function toggleFullscreen() {
		const container = mapContext.map?.getContainer();
		if (!container) return;
		if (document.fullscreenElement) {
			void document.exitFullscreen();
		} else {
			void container.requestFullscreen();
		}
	}

	// Turns the compass needle with the map's bearing and pitch.
	const compass: Attachment<SVGSVGElement> = (svg) => {
		const map = mapContext.map;
		if (!map) return;

		const updateRotation = () => {
			svg.style.transform = `rotateX(${map.getPitch()}deg) rotateZ(${-map.getBearing()}deg)`;
		};

		map.on('rotate', updateRotation);
		map.on('pitch', updateRotation);
		updateRotation();

		return () => {
			map.off('rotate', updateRotation);
			map.off('pitch', updateRotation);
		};
	};
</script>

<div class={cn('absolute z-10 flex flex-col gap-1.5', positionClasses[position], className)}>
	{#if showZoom}
		<div class={groupClass}>
			<button type="button" class={buttonClass} aria-label="Zoom in" onclick={() => zoomBy(1)}>
				<PlusIcon class="size-4" />
			</button>
			<button type="button" class={buttonClass} aria-label="Zoom out" onclick={() => zoomBy(-1)}>
				<MinusIcon class="size-4" />
			</button>
		</div>
	{/if}
	{#if showCompass}
		<div class={groupClass}>
			<button
				type="button"
				class={buttonClass}
				aria-label="Reset bearing to north"
				onclick={resetBearing}
			>
				<svg
					{@attach compass}
					viewBox="0 0 24 24"
					class="size-5 transition-transform duration-200"
					style="transform-style: preserve-3d"
				>
					<path d="M12 2L16 12H12V2Z" class="fill-red-500" />
					<path d="M12 2L8 12H12V2Z" class="fill-red-300" />
					<path d="M12 22L16 12H12V22Z" class="fill-muted-foreground/60" />
					<path d="M12 22L8 12H12V22Z" class="fill-muted-foreground/30" />
				</svg>
			</button>
		</div>
	{/if}
	{#if showLocate}
		<div class={groupClass}>
			<button
				type="button"
				class={cn(
					buttonClass,
					waitingForLocation && 'pointer-events-none cursor-not-allowed opacity-50'
				)}
				aria-label="Find my location"
				disabled={waitingForLocation}
				onclick={locate}
			>
				{#if waitingForLocation}
					<LoaderCircleIcon class="size-4 animate-spin" />
				{:else}
					<LocateIcon class="size-4" />
				{/if}
			</button>
		</div>
	{/if}
	{#if showFullscreen}
		<div class={groupClass}>
			<button
				type="button"
				class={buttonClass}
				aria-label="Toggle fullscreen"
				onclick={toggleFullscreen}
			>
				<MaximizeIcon class="size-4" />
			</button>
		</div>
	{/if}
</div>
