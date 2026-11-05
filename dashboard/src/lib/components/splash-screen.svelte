<script lang="ts">
	import { onMount } from 'svelte';
	import { useIsFetching } from '@tanstack/svelte-query';

	import favicon from '$lib/assets/favicon.svg';
	import { bootProgress } from '$lib/state/boot-progress.svelte';

	// Number of queries currently in flight — the real signal for the content phase.
	const isFetching = useIsFetching();

	let displayed = $state(0); // 0..1 shown on the bar (eased toward the real target)
	let fading = $state(false); // overlay opacity transitioning to 0
	let hidden = $state(false); // removed from the DOM after the fade
	let raf = 0;

	const percent = $derived(Math.round(displayed * 100));

	// The landing route's data is "done" once the tenant list has settled AND nothing
	// is fetching. A short settle avoids a false 100% between two chained requests.
	$effect(() => {
		if (bootProgress.done.tenants && isFetching.current === 0) {
			const t = setTimeout(() => bootProgress.mark('content'), 150);
			return () => clearTimeout(t);
		}
	});

	function frame() {
		// Ceiling is driven by real milestones (`target`); a tiny creep keeps the bar
		// from looking stuck while a slow request is in flight, capped below 92% until
		// everything is genuinely done.
		const ceiling = bootProgress.complete
			? 1
			: Math.min(0.92, Math.max(bootProgress.target, displayed + 0.0015));
		displayed += (ceiling - displayed) * 0.09;

		if (bootProgress.complete && displayed > 0.995) {
			displayed = 1;
			// Let the full bar register, then play the exit: overlay lifts + dissolves
			// while the app scales in behind it (bootProgress.revealed drives that).
			setTimeout(() => {
				fading = true;
				bootProgress.reveal();
				// Safety net: the overlay normally removes itself on `animationend`, but if
				// that event never fires (reduced motion quirks, backgrounded tab), force it
				// hidden after the exit animation's duration so the app is never blocked.
				setTimeout(() => (hidden = true), 900);
			}, 180);
			return; // stop the loop
		}
		raf = requestAnimationFrame(frame);
	}

	onMount(() => {
		// Hard cap: never trap the user behind the splash if a request hangs.
		const cap = setTimeout(() => bootProgress.finishAll(), 8000);
		raf = requestAnimationFrame(frame);
		return () => {
			cancelAnimationFrame(raf);
			clearTimeout(cap);
		};
	});
</script>

{#if !hidden}
	<div
		class="splash"
		class:fading
		role="status"
		aria-live="polite"
		aria-label="Loading dashboard"
		onanimationend={(e) => {
			// Only the overlay's own exit animation ends the splash (ignore children).
			if (fading && (e.animationName === 'splash-out' || e.animationName === 'splash-fade'))
				hidden = true;
		}}
	>
		<div class="splash-inner">
			<div class="splash-logo-ring">
				<img src={favicon} alt="" class="splash-logo" />
			</div>
			<div class="splash-title">GateKeeper</div>
			<div class="splash-bar" aria-hidden="true">
				<div class="splash-fill" style="width: {displayed * 100}%"></div>
			</div>
			<div class="splash-meta">
				<span>Loading your workspace…</span>
				<span class="splash-percent">{percent}%</span>
			</div>
		</div>
	</div>
{/if}

<style>
	.splash {
		position: fixed;
		inset: 0;
		z-index: 200;
		display: grid;
		place-items: center;
		background: var(--background);
		color: var(--foreground);
		will-change: opacity, transform, filter;
	}
	/* Exit: the overlay lifts, scales up a touch, and dissolves into a soft blur —
	   revealing the (already-themed) app, which scales in behind it. */
	.splash.fading {
		pointer-events: none;
		animation: splash-out 0.62s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}
	.splash.fading .splash-inner {
		animation: splash-content-out 0.4s cubic-bezier(0.22, 1, 0.36, 1) forwards;
	}

	.splash-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.1rem;
		width: min(320px, 78vw);
		padding: 2rem;
	}

	/* Logo with a soft, breathing brand-colored glow behind it. */
	.splash-logo-ring {
		position: relative;
		display: grid;
		place-items: center;
	}
	.splash-logo-ring::before {
		content: '';
		position: absolute;
		width: 5.5rem;
		height: 5.5rem;
		border-radius: 9999px;
		background: radial-gradient(
			circle,
			color-mix(in oklch, var(--primary) 45%, transparent),
			transparent 70%
		);
		filter: blur(6px);
		animation: splash-glow 2s ease-in-out infinite;
	}
	.splash-logo {
		position: relative;
		width: 3rem;
		height: 3rem;
		animation: splash-pulse 1.8s ease-in-out infinite;
	}

	.splash-title {
		font-size: 1.05rem;
		font-weight: 600;
		letter-spacing: -0.01em;
	}

	.splash-bar {
		width: 100%;
		height: 6px;
		border-radius: 9999px;
		background: var(--muted);
		overflow: hidden;
	}
	.splash-fill {
		position: relative;
		height: 100%;
		border-radius: 9999px;
		background: var(--primary);
		overflow: hidden;
		transition: width 0.18s ease-out;
	}
	/* Moving sheen sweeping across the filled portion. */
	.splash-fill::after {
		content: '';
		position: absolute;
		inset: 0;
		background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.4), transparent);
		transform: translateX(-100%);
		animation: splash-shimmer 1.4s ease-in-out infinite;
	}

	.splash-meta {
		display: flex;
		width: 100%;
		justify-content: space-between;
		font-size: 0.75rem;
		color: var(--muted-foreground);
		font-variant-numeric: tabular-nums;
	}
	.splash-percent {
		font-weight: 500;
		color: var(--foreground);
	}

	@keyframes splash-pulse {
		0%,
		100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.72;
			transform: scale(0.94);
		}
	}
	@keyframes splash-glow {
		0%,
		100% {
			opacity: 0.55;
			transform: scale(0.9);
		}
		50% {
			opacity: 1;
			transform: scale(1.12);
		}
	}
	@keyframes splash-shimmer {
		0% {
			transform: translateX(-100%);
		}
		60%,
		100% {
			transform: translateX(100%);
		}
	}
	@keyframes splash-content-out {
		to {
			opacity: 0;
			transform: translateY(-14px) scale(0.96);
		}
	}
	@keyframes splash-out {
		0% {
			opacity: 1;
			transform: scale(1);
			filter: blur(0);
		}
		100% {
			opacity: 0;
			transform: scale(1.07);
			filter: blur(10px);
		}
	}
	@keyframes splash-fade {
		to {
			opacity: 0;
		}
	}

	/* Motion-safe fallback: no lift/blur/shimmer — just a clean fade. */
	@media (prefers-reduced-motion: reduce) {
		.splash-logo,
		.splash-logo-ring::before,
		.splash-fill::after {
			animation: none;
		}
		.splash.fading {
			animation: splash-fade 0.3s ease forwards;
		}
		.splash.fading .splash-inner {
			animation: none;
		}
	}
</style>
