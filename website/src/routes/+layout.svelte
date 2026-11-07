<script lang="ts">
	import { page } from '$app/state';
	import { ModeWatcher } from 'mode-watcher';
	import { locales, localizeHref } from '$lib/paraglide/runtime';
	import { Toaster } from '$lib/components/ui/sonner/index.js';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<ModeWatcher />
<Toaster richColors />

{@render children()}

<!-- Docs are English-only and prerendered. Without this guard the prerender crawler follows
     these links and bakes a duplicate /pt-br/docs/* copy of every page, with identical English
     content. `page.route.id` is the rerouted id, so this also covers /pt-br/docs/* at runtime. -->
{#if !page.route.id?.startsWith('/docs')}
	<div style="display:none">
		{#each locales as locale (locale)}
			<a href={localizeHref(page.url.pathname, { locale })}>{locale}</a>
		{/each}
	</div>
{/if}
