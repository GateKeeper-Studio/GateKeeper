<script lang="ts">
	import type { Snippet } from 'svelte';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';
	import FormPanel from '../form-panel.svelte';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';
	import type { ResolvedLoginAppearance } from '$lib/types/login-appearance';

	let {
		client,
		appearance,
		termsAndConditionsEnabled = true,
		children
	}: {
		client: ClientAuthData | null;
		appearance: ResolvedLoginAppearance;
		termsAndConditionsEnabled?: boolean;
		children: Snippet;
	} = $props();

	const heroOnRight = $derived(appearance.heroPosition === 'right');
</script>

<div
	class="relative flex h-screen flex-col items-center justify-center lg:grid lg:max-w-none lg:grid-cols-2 lg:px-0"
>
	<!-- Hero / brand panel (hidden on small screens). Order flips for hero-on-right. -->
	<div
		class="relative hidden h-screen flex-col justify-between bg-primary p-10 text-white lg:flex dark:border-r"
		class:lg:order-2={heroOnRight}
	>
		{#if appearance.backgroundImageUri}
			<!-- Rendered as an <img> layer (never a CSS url()), so the validated http(s)
			     URL cannot break out of a style context. -->
			<img
				src={appearance.backgroundImageUri}
				alt=""
				class="absolute inset-0 h-full w-full object-cover"
			/>
			<div class="absolute inset-0 bg-black/40"></div>
		{/if}

		<div class="relative z-20 flex items-center gap-2 text-lg font-medium">
			{#if appearance.logoUri}
				<img src={appearance.logoUri} alt="" class="h-8 w-auto object-contain" />
			{/if}
			{client?.name}
		</div>

		{#if appearance.heading || appearance.subtext}
			<div class="relative z-20">
				{#if appearance.heading}
					<p class="text-2xl font-semibold tracking-tight">{appearance.heading}</p>
				{/if}
				{#if appearance.subtext}
					<p class="mt-2 text-sm text-white/80">{appearance.subtext}</p>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Form panel -->
	<div class="relative flex h-full w-full flex-1 items-center justify-center overflow-auto">
		<div class="flex w-full justify-center p-6 sm:p-8">
			<span class="absolute right-4 bottom-4 flex items-center gap-2">
				<LanguageSwitcher allowedLanguages={client?.allowedLanguages} />
				<ThemeToggle />
			</span>

			<FormPanel {client} {appearance} {termsAndConditionsEnabled}>
				{@render children()}
			</FormPanel>
		</div>
	</div>
</div>
