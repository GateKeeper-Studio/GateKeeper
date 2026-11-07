<script lang="ts">
	import MenuIcon from '@lucide/svelte/icons/menu';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';

	import { resolve } from '$app/paths';

	import { Button, buttonVariants } from '$lib/components/ui/button/index.js';
	import * as Sheet from '$lib/components/ui/sheet/index.js';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';
	import { dashboardHref } from '$lib/external-urls';
	import { m } from '$lib/paraglide/messages.js';

	let mobileOpen = $state(false);

	// Hash links scroll within the page; /docs is a real route (base-path aware).
	const links = $derived([
		{ label: m.landing_nav_features(), href: '#features' },
		{ label: m.landing_nav_developers(), href: '#developers' },
		{ label: m.landing_nav_docs(), href: resolve('/docs') }
	]);
</script>

<!-- Nav hrefs are static hash anchors (in-page) or routes pre-resolved above. -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->
<header class="sticky top-0 z-50 border-b border-border/60 bg-background/70 backdrop-blur-xl">
	<div class="mx-auto flex h-16 max-w-7xl items-center gap-4 px-4 sm:px-6">
		<a href={resolve('/')} class="font-semibold tracking-tight flex gap-4 text-lg items-center">
			<img src="/src/lib/assets/logo-without-text.svg" alt="GateKeeper Logo" width="36"/>

			<span class="text-lg">GateKeeper</span>
		</a>

		<nav class="ml-6 hidden items-center gap-1 md:flex">
			{#each links as link (link.href)}
				<a
					href={link.href}
					class="rounded-md px-3 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
				>
					{link.label}
				</a>
			{/each}
		</nav>

		<div class="ml-auto flex items-center gap-2">
			<LanguageSwitcher />
			<ThemeToggle />
			<Button
				href="/docs"
				size="sm"
				class="hidden bg-gradient-to-r from-brand to-brand-2 text-brand-foreground hover:opacity-90 sm:inline-flex"
			>
				{m.landing_nav_get_started()}
				<ArrowRightIcon class="size-4" />
			</Button>

			<Sheet.Root bind:open={mobileOpen}>
				<Sheet.Trigger
					class={buttonVariants({ variant: 'ghost', size: 'icon', class: 'md:hidden' })}
				>
					<MenuIcon class="size-5" />
					<span class="sr-only">{m.landing_nav_open_menu()}</span>
				</Sheet.Trigger>

				<Sheet.Content side="right" class="w-72 p-6">
					<Sheet.Title class="sr-only">{m.landing_nav_mobile_menu_title()}</Sheet.Title>
					
					<Sheet.Description class="sr-only">
						{m.landing_nav_mobile_menu_description()}
					</Sheet.Description>
					
					<nav class="mt-8 flex flex-col gap-1">
						{#each links as link (link.href)}
							<a
								href={link.href}
								onclick={() => (mobileOpen = false)}
								class="rounded-md px-3 py-2.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
							>
								{link.label}
							</a>
						{/each}
	
						<Button
							href="/docs"
							class="bg-linear-to-r from-brand to-brand-2 text-brand-foreground"
						>
							{m.landing_nav_get_started()}
						</Button>
					</nav>
				</Sheet.Content>
			</Sheet.Root>
		</div>
	</div>
</header>
