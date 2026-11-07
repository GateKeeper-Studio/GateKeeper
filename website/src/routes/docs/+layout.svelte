<script lang="ts">
	import MenuIcon from '@lucide/svelte/icons/menu';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

	import { page } from '$app/state';
	import { deLocalizeUrl } from '$lib/paraglide/runtime';

	import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';

	import DocsSearch from '$lib/components/docs/docs-search.svelte';
	import DocsNavList from '$lib/components/docs/docs-nav-list.svelte';
	import { docsNav, flatDocsNav } from '$lib/docs/nav';
	import { dashboardHref } from '$lib/external-urls';

	let { children } = $props();

	let mobileOpen = $state(false);

	// deLocalizeUrl keeps /pt-br/docs/x matching the /docs/x nav entry.
	const current = $derived(deLocalizeUrl(page.url).pathname.replace(/\/$/, '') || '/docs');

	const position = $derived(flatDocsNav.findIndex((item) => item.href === current));
	const previous = $derived(position > 0 ? flatDocsNav[position - 1] : null);
	const next = $derived(
		position >= 0 && position < flatDocsNav.length - 1 ? flatDocsNav[position + 1] : null
	);
</script>

<div class="min-h-screen bg-background">
	<header class="sticky top-0 z-40 border-b bg-background/80 backdrop-blur">
		<div class="mx-auto flex h-16 max-w-7xl items-center gap-4 px-4 sm:px-6">
			<Sheet.Root bind:open={mobileOpen}>
				<Sheet.Trigger
					class={buttonVariants({ variant: 'ghost', size: 'icon', class: 'lg:hidden' })}
				>
					<MenuIcon class="size-5" />
					<span class="sr-only">Open navigation</span>
				</Sheet.Trigger>
				<Sheet.Content side="left" class="w-72 overflow-y-auto p-6">
					<DocsNavList sections={docsNav} {current} onnavigate={() => (mobileOpen = false)} />
				</Sheet.Content>
			</Sheet.Root>

			<a href="/docs" class="font-semibold tracking-tight flex gap-4 text-lg items-center">
				<img src="/src/lib/assets/logo-without-text.svg" alt="GateKeeper Logo" width="36"/>
				<span class="flex gap-2 text-lg items-center">
					GateKeeper <span class="bg-blue-600 px-2 rounded-md text-white text-sm py-1">Docs</span>
				</span>
			</a>

			<div class="ml-auto flex items-center gap-2">
				<DocsSearch />
				<LanguageSwitcher />
				<ThemeToggle />
				<Button href="/" variant="ghost" size="sm" class="hidden sm:inline-flex">
					Home
				</Button>
			</div>
		</div>
	</header>

	<div class="mx-auto flex max-w-7xl gap-10 px-4 py-10">
		<aside class="sticky top-24 hidden h-[calc(100vh-8rem)] w-60 shrink-0 overflow-y-auto lg:block">
			<DocsNavList sections={docsNav} {current} />
		</aside>

		<main class="min-w-0 flex-1">
			{@render children()}

			<Separator class="my-10" />

			<nav class="flex items-center justify-between gap-4">
				{#if previous}
					<Button
						href={previous.href}
						variant="outline"
						class="h-auto flex-col items-start gap-1 py-3"
					>
						<span class="flex items-center gap-1 text-xs text-muted-foreground">
							<ChevronLeftIcon class="size-3" />
							Previous
						</span>
						<span>{previous.title}</span>
					</Button>
				{:else}
					<span></span>
				{/if}

				{#if next}
					<Button
						href={next.href}
						variant="outline"
						class="ml-auto h-auto flex-col items-end gap-1 py-3"
					>
						<span class="flex items-center gap-1 text-xs text-muted-foreground">
							Next
							<ChevronRightIcon class="size-3" />
						</span>
						<span>{next.title}</span>
					</Button>
				{/if}
			</nav>
		</main>
	</div>
</div>
