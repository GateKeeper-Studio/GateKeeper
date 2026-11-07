<script lang="ts">
	import ImageIcon from '@lucide/svelte/icons/image';
	import type { Component } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		/** Caption shown under the icon, e.g. "Dashboard screenshot". */
		label?: string;
		/** Optional lucide icon component (defaults to Image). */
		icon?: Component;
		/** Tailwind aspect utility, e.g. "aspect-video" (default) or "aspect-square". */
		aspect?: string;
		class?: string;
	};

	let {
		label = m.landing_placeholder_default_label(),
		icon: Icon = ImageIcon,
		aspect = 'aspect-video',
		class: className
	}: Props = $props();
</script>

<!-- Clearly-marked stand-in for a future product screenshot. Swap for an <img>. -->
<div
	class={cn(
		'relative flex w-full items-center justify-center overflow-hidden rounded-xl border border-dashed border-border bg-muted/60',
		aspect,
		className
	)}
	role="img"
	aria-label={label}
>
	<!-- diagonal hatch to read unmistakably as a placeholder -->
	<div
		class="pointer-events-none absolute inset-0 [background-image:repeating-linear-gradient(45deg,transparent,transparent_11px,var(--color-border)_11px,var(--color-border)_12px)] opacity-[0.35]"
	></div>
	<!-- shimmer sweep -->
	<div
		class="pointer-events-none absolute inset-0 -translate-x-full animate-[shimmer_2.4s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-foreground/[0.05] to-transparent"
	></div>

	<div
		class="relative z-10 flex flex-col items-center gap-2 px-4 text-center text-muted-foreground"
	>
		<div
			class="flex size-11 items-center justify-center rounded-full bg-background/70 ring-1 ring-border"
		>
			<Icon class="size-5" />
		</div>
		<span class="text-xs font-medium tracking-wide uppercase">{label}</span>
	</div>
</div>

<style>
	@keyframes shimmer {
		0% {
			transform: translateX(-100%);
		}
		60%,
		100% {
			transform: translateX(100%);
		}
	}
</style>
