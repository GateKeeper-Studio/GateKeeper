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
</script>

<!-- Card: the form floats in an elevated card over a full-bleed background
     (image when provided, otherwise the brand primary color). -->
<div class="relative flex min-h-screen w-full items-center justify-center overflow-auto p-6 sm:p-8">
	<div class="absolute inset-0 bg-primary">
		{#if appearance.backgroundImageUri}
			<img
				src={appearance.backgroundImageUri}
				alt=""
				class="absolute inset-0 h-full w-full object-cover"
			/>
			<div class="absolute inset-0 bg-black/40"></div>
		{/if}
	</div>

	<span class="absolute right-4 bottom-4 z-20 flex items-center gap-2">
		<LanguageSwitcher allowedLanguages={client?.allowedLanguages} />
		<ThemeToggle />
	</span>

	<div class="relative z-10 w-full max-w-md">
		{#if appearance.heading || appearance.subtext}
			<div class="mb-4 text-center text-white">
				{#if appearance.heading}
					<h2 class="text-2xl font-semibold tracking-tight">{appearance.heading}</h2>
				{/if}
				{#if appearance.subtext}
					<p class="mt-1 text-sm text-white/80">{appearance.subtext}</p>
				{/if}
			</div>
		{/if}

		<div class="rounded-xl border bg-background p-6 shadow-xl sm:p-8">
			<FormPanel {client} {appearance} {termsAndConditionsEnabled}>
				{@render children()}
			</FormPanel>
		</div>
	</div>
</div>
