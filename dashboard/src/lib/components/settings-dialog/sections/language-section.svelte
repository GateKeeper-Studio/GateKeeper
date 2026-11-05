<script lang="ts">
	import CheckIcon from '@lucide/svelte/icons/check';

	import { Button } from '$lib/components/ui/button/index.js';
	import { getLocale, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { LOCALES } from '$lib/i18n/locales';
	import { m } from '$lib/paraglide/messages.js';

	// Language is a personal, per-browser preference (cookie-backed via Paraglide) —
	// unlike the organization-wide Appearance section, nothing here is saved to the tenant.
	const current = getLocale();

	function selectLocale(code: Locale) {
		if (code === current) return;
		setLocale(code); // writes the PARAGLIDE_LOCALE cookie and reloads
	}
</script>

<header>
	<h1 class="text-lg font-semibold tracking-tight">{m.settings_language_title()}</h1>
	<p class="text-sm text-muted-foreground">{m.settings_language_description()}</p>
</header>

<div class="mt-6 grid max-w-md gap-2">
	{#each LOCALES as locale (locale.code)}
		<Button
			type="button"
			variant={locale.code === current ? 'default' : 'outline'}
			class="h-11 justify-start gap-2"
			onclick={() => selectLocale(locale.code)}
		>
			<CheckIcon class={locale.code === current ? 'opacity-100' : 'opacity-0'} />
			{locale.nativeLabel}
		</Button>
	{/each}
</div>
