<script lang="ts">
	import { reveal } from '$lib/actions/reveal';
	import { m } from '$lib/paraglide/messages.js';

	// Placeholder company names — swap for real customer logos (SVGs).
	const logos = ['Northwind', 'Acme Corp', 'Globex', 'Initech', 'Umbrella', 'Hooli'];
</script>

<section class="border-y border-border/60 bg-muted/30">
	<div class="mx-auto max-w-7xl px-4 py-10 sm:px-6" use:reveal>
		<p class="text-center text-sm font-medium text-muted-foreground">
			{m.landing_logo_cloud_heading()}
		</p>
		<div class="mt-6 flex flex-wrap items-center justify-center gap-x-10 gap-y-6">
			{#each logos as name (name)}
				<!-- placeholder wordmark logo -->
				<div
					class="flex items-center gap-2 text-lg font-semibold text-muted-foreground/60 grayscale transition-all hover:text-foreground hover:grayscale-0"
				>
					<span class="size-5 rounded-md bg-gradient-to-br from-brand/50 to-brand-2/50"></span>
					{name}
				</div>
			{/each}
		</div>
	</div>
</section>
