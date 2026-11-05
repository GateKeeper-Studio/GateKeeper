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

<div
	class="relative flex min-h-screen w-full flex-col items-center justify-center overflow-auto p-6 sm:p-8"
>
	<span class="absolute right-4 bottom-4 flex items-center gap-2">
		<LanguageSwitcher allowedLanguages={client?.allowedLanguages} />
		<ThemeToggle />
	</span>

	{#if appearance.heading || appearance.subtext}
		<div class="mb-6 max-w-sm text-center">
			{#if appearance.heading}
				<h2 class="text-2xl font-semibold tracking-tight">{appearance.heading}</h2>
			{/if}
			{#if appearance.subtext}
				<p class="mt-1 text-sm text-muted-foreground">{appearance.subtext}</p>
			{/if}
		</div>
	{/if}

	<FormPanel {client} {appearance} {termsAndConditionsEnabled}>
		{@render children()}
	</FormPanel>
</div>
