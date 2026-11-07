<script lang="ts">
	import LandingNav from '$lib/components/landing/landing-nav.svelte';
	import Hero from '$lib/components/landing/hero.svelte';
	import LogoCloud from '$lib/components/landing/logo-cloud.svelte';
	import Features from '$lib/components/landing/features.svelte';
	import CodeSample from '$lib/components/landing/code-sample.svelte';
	import StatsBand from '$lib/components/landing/stats-band.svelte';
	import Cta from '$lib/components/landing/cta.svelte';
	import SiteFooter from '$lib/components/landing/site-footer.svelte';
	import { m } from '$lib/paraglide/messages.js';
</script>

<svelte:head>
	<title>{m.landing_meta_title()}</title>
	<meta name="description" content={m.landing_meta_description()} />
	<meta property="og:title" content={m.landing_meta_title()} />
	<meta property="og:description" content={m.landing_meta_description()} />
	<meta property="og:type" content="website" />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<div class="min-h-screen bg-background">
	<LandingNav />

	<main>
		<Hero />
		<LogoCloud />
		<Features />
		<CodeSample />
		<StatsBand />
		<Cta />
	</main>

	<SiteFooter />
</div>
