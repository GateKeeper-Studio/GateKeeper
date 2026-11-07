<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import CpuIcon from '@lucide/svelte/icons/cpu';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import GaugeIcon from '@lucide/svelte/icons/gauge';

	import { Button } from '$lib/components/ui/button/index.js';
	import { dashboardHref } from '$lib/external-urls';
	import ImagePlaceholder from './image-placeholder.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const trustBadges = $derived([
		{ icon: CpuIcon, label: m.landing_hero_badge_rust() },
		{ icon: ShieldCheckIcon, label: m.landing_hero_badge_memory_safe() },
		{ icon: GaugeIcon, label: m.landing_hero_badge_no_gc() }
	]);
</script>

<section class="relative overflow-hidden">
	<!-- Aurora glow background -->
	<div class="pointer-events-none absolute inset-0 -z-10 overflow-hidden" aria-hidden="true">
		<div
			class="gk-aurora absolute -top-40 left-1/2 size-[42rem] -translate-x-1/2 rounded-full bg-brand/25 blur-[120px]"
		></div>
		<div
			class="gk-aurora absolute top-10 -right-32 size-[34rem] rounded-full bg-brand-2/25 blur-[120px] [animation-delay:-4s]"
		></div>
		<div
			class="gk-aurora absolute -bottom-40 -left-24 size-[32rem] rounded-full bg-brand/20 blur-[120px] [animation-delay:-8s]"
		></div>
		<!-- faint grid -->
		<div
			class="absolute inset-0 [background-image:linear-gradient(to_right,var(--color-foreground)_1px,transparent_1px),linear-gradient(to_bottom,var(--color-foreground)_1px,transparent_1px)] [mask-image:radial-gradient(ellipse_at_center,black,transparent_75%)] [background-size:56px_56px] opacity-[0.04]"
		></div>
	</div>

	<div class="mx-auto max-w-7xl px-4 pt-20 pb-16 sm:px-6 sm:pt-28">
		<div class="mx-auto max-w-3xl text-center">
			<div
				class="mb-6 inline-flex animate-in items-center gap-2 rounded-full border border-border bg-background/60 px-4 py-1.5 text-sm text-muted-foreground backdrop-blur duration-700 fade-in slide-in-from-bottom-3"
			>
				<SparklesIcon class="size-3.5 text-brand" />
				{m.landing_hero_badge()}
			</div>

			<h1
				class="animate-in text-4xl font-semibold tracking-tight text-balance duration-700 [animation-delay:80ms] fade-in slide-in-from-bottom-4 sm:text-6xl"
			>
				{m.landing_hero_title_line1()}
				<span class="bg-gradient-to-r from-brand to-brand-2 bg-clip-text text-transparent">
					{m.landing_hero_title_highlight()}
				</span>
			</h1>

			<p
				class="mx-auto mt-6 max-w-2xl animate-in text-lg text-pretty text-muted-foreground duration-700 [animation-delay:160ms] fade-in slide-in-from-bottom-4"
			>
				{m.landing_hero_subtitle()}
			</p>

			<div
				class="mt-9 flex animate-in flex-col items-center justify-center gap-3 duration-700 [animation-delay:240ms] fade-in slide-in-from-bottom-4 sm:flex-row"
			>
				<Button
					href="/docs"
					size="lg"
					class="w-full bg-gradient-to-r from-brand to-brand-2 text-brand-foreground shadow-lg shadow-brand/20 transition-opacity hover:opacity-90 sm:w-auto"
				>
					<BookOpenIcon class="size-4" />
					{m.landing_hero_cta_secondary()}
				</Button>
			</div>

			<div
				class="mt-8 flex animate-in flex-wrap items-center justify-center gap-2 duration-700 [animation-delay:300ms] fade-in slide-in-from-bottom-4"
			>
				{#each trustBadges as badge (badge.label)}
					{@const Icon = badge.icon}
					<span
						class="inline-flex items-center gap-1.5 rounded-full border border-border bg-background/60 px-3 py-1 text-xs font-medium text-muted-foreground backdrop-blur"
					>
						<Icon class="size-3.5 text-brand" />
						{badge.label}
					</span>
				{/each}
			</div>

			<p class="mt-4 text-sm text-muted-foreground">{m.landing_hero_no_credit_card()}</p>
		</div>

		<!-- Product mockup: browser frame wrapping a screenshot placeholder -->
		<div
			class="mx-auto mt-16 max-w-5xl animate-in duration-1000 [animation-delay:320ms] fade-in slide-in-from-bottom-8"
		>
			<div
				class="rounded-2xl border border-border bg-card/70 p-2 shadow-2xl shadow-brand/10 backdrop-blur"
			>
				<div class="flex items-center gap-2 px-3 py-2">
					<span class="size-3 rounded-full bg-destructive/60"></span>
					<span class="size-3 rounded-full bg-chart-1"></span>
					<span class="size-3 rounded-full bg-brand-2/70"></span>
					<div
						class="mx-auto flex items-center gap-1.5 rounded-md bg-muted px-3 py-1 text-xs text-muted-foreground"
					>
						<LayoutDashboardIcon class="size-3" />
						app.gatekeeper.dev
					</div>
				</div>
				<ImagePlaceholder
					label={m.landing_hero_mockup_screenshot_label()}
					icon={LayoutDashboardIcon}
					class="rounded-xl border-border/60"
				/>
			</div>
		</div>
	</div>
</section>
