<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';

	import { Button } from '$lib/components/ui/button/index.js';
	import { reveal } from '$lib/actions/reveal';
	import { dashboardHref } from '$lib/external-urls';
	import { m } from '$lib/paraglide/messages.js';
</script>

<section class="mx-auto max-w-7xl px-4 py-24 sm:px-6">
	<div
		use:reveal
		class="relative overflow-hidden rounded-3xl border border-border bg-gradient-to-br from-brand to-brand-2 px-6 py-16 text-center shadow-2xl shadow-brand/20 sm:px-16"
	>
		<!-- decorative glow -->
		<div
			class="pointer-events-none absolute inset-0 [background-image:radial-gradient(circle_at_20%_20%,white,transparent_40%),radial-gradient(circle_at_80%_80%,white,transparent_40%)] opacity-30"
			aria-hidden="true"
		></div>

		<div class="relative">
			<h2 class="text-3xl font-semibold tracking-tight text-balance text-white sm:text-4xl">
				{m.landing_cta_heading()}
			</h2>
			<p class="mx-auto mt-4 max-w-xl text-lg text-pretty text-white/80">
				{m.landing_cta_subheading()}
			</p>
			<div class="mt-9 flex flex-col items-center justify-center gap-3 sm:flex-row">
				<Button
					href={dashboardHref('/login')}
					size="lg"
					class="w-full bg-white text-brand hover:bg-white/90 sm:w-auto"
				>
					{m.landing_cta_primary()}
					<ArrowRightIcon class="size-4" />
				</Button>
				<Button
					href="/docs"
					size="lg"
					variant="outline"
					class="w-full border-white/40 bg-transparent text-white hover:bg-white/10 hover:text-white sm:w-auto"
				>
					{m.landing_cta_secondary()}
				</Button>
			</div>
		</div>
	</div>
</section>
