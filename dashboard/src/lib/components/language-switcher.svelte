<script lang="ts">
	import LanguagesIcon from '@lucide/svelte/icons/languages';
	import CheckIcon from '@lucide/svelte/icons/check';

	import { getLocale, setLocale, type Locale } from '$lib/paraglide/runtime';
	import { LOCALES } from '$lib/i18n/locales';
	import { m } from '$lib/paraglide/messages.js';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	// When provided (hosted auth pages), only offer the languages this OAuth
	// client allows — the dashboard's own (non-auth) usage passes nothing and
	// keeps offering every supported locale.
	let { allowedLanguages }: { allowedLanguages?: string[] } = $props();

	const locales = $derived(
		allowedLanguages ? LOCALES.filter((l) => allowedLanguages.includes(l.code)) : LOCALES
	);

	// Read once on render. `setLocale` reloads the page (Paraglide's default), so the
	// active-marker is always correct after a switch without extra reactivity.
	const current = getLocale();

	function selectLocale(code: Locale) {
		if (code === current) return;
		setLocale(code); // writes the PARAGLIDE_LOCALE cookie and reloads
	}
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="outline" size="icon">
				<LanguagesIcon class="h-[1.2rem] w-[1.2rem]" />
				<span class="sr-only">{m.language_switcher_label()}</span>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Content align="end">
		{#each locales as locale (locale.code)}
			<DropdownMenu.Item onclick={() => selectLocale(locale.code)}>
				<CheckIcon class={locale.code === current ? 'opacity-100' : 'opacity-0'} />
				{locale.nativeLabel}
			</DropdownMenu.Item>
		{/each}
	</DropdownMenu.Content>
</DropdownMenu.Root>
