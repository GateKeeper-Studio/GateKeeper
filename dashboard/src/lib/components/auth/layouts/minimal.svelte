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

<!-- Minimal: no hero, no brand copy — just the (optional) logo and the form,
     sitting slightly above center for a clean, neutral look. -->
<div
	class="relative flex min-h-screen w-full flex-col items-center overflow-auto px-6 pt-[12vh] pb-8"
>
	<span class="absolute top-4 right-4 flex items-center gap-2">
		<LanguageSwitcher allowedLanguages={client?.allowedLanguages} />
		<ThemeToggle />
	</span>

	<FormPanel {client} {appearance} {termsAndConditionsEnabled}>
		{@render children()}
	</FormPanel>
</div>
