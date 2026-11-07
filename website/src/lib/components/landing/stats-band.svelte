<script lang="ts">
	import { Tween } from 'svelte/motion';
	import { cubicOut } from 'svelte/easing';
	import { m } from '$lib/paraglide/messages.js';

	type Stat = { to: number; label: string; format: (n: number) => string };

	// Placeholder metrics — swap for real numbers.
	const stats: Stat[] = $derived([
		{
			to: 2400,
			label: m.landing_stats_active_tenants(),
			format: (n) => `${Math.round(n).toLocaleString()}+`
		},
		{ to: 180, label: m.landing_stats_tokens_issued(), format: (n) => `${Math.round(n)}M+` },
		{ to: 99.99, label: m.landing_stats_uptime_sla(), format: (n) => `${n.toFixed(2)}%` },
		{ to: 40, label: m.landing_stats_median_latency(), format: (n) => `${Math.round(n)}ms` }
	]);

	const reduce =
		typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;

	const tweens = stats.map(() => new Tween(0, { duration: reduce ? 0 : 1600, easing: cubicOut }));

	let sectionEl: HTMLElement | undefined = $state();

	$effect(() => {
		if (!sectionEl || typeof IntersectionObserver === 'undefined') {
			// no observer available — show final values
			stats.forEach((s, i) => tweens[i].set(s.to, { duration: 0 }));
			return;
		}
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0]?.isIntersecting) {
					stats.forEach((s, i) => (tweens[i].target = s.to));
					observer.disconnect();
				}
			},
			{ threshold: 0.3 }
		);
		observer.observe(sectionEl);
		return () => observer.disconnect();
	});
</script>

<section
	bind:this={sectionEl}
	class="relative overflow-hidden border-y border-border/60 bg-gradient-to-br from-brand/10 via-background to-brand-2/10"
>
	<div class="mx-auto grid max-w-7xl grid-cols-2 gap-8 px-4 py-16 sm:px-6 lg:grid-cols-4">
		{#each stats as stat, i (stat.label)}
			<div class="text-center">
				<div
					class="bg-gradient-to-r from-brand to-brand-2 bg-clip-text text-4xl font-semibold tracking-tight text-transparent tabular-nums sm:text-5xl"
				>
					{stat.format(tweens[i].current)}
				</div>
				<div class="mt-2 text-sm text-muted-foreground">{stat.label}</div>
			</div>
		{/each}
	</div>
</section>
