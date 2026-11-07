<script lang="ts">
	import { page } from '$app/state';
	import { ModeWatcher } from 'mode-watcher';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import { getLocale, getTextDirection, locales, localizeHref } from '$lib/paraglide/runtime';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	const queryClient = new QueryClient();

	// There is no server render to stamp the locale into <html>, so set it here.
	$effect(() => {
		const locale = getLocale();
		document.documentElement.lang = locale;
		document.documentElement.dir = getTextDirection(locale);
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<ModeWatcher />
<Toaster richColors />

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

<div style="display:none">
	{#each locales as locale (locale)}
		<a href={localizeHref(page.url.pathname, { locale })}>{locale}</a>
	{/each}
</div>
