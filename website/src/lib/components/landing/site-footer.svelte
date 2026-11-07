<script lang="ts">
	import { resolve } from '$app/paths';

	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';
	import { dashboardHref } from '$lib/external-urls';
	import { m } from '$lib/paraglide/messages.js';

	const columns = $derived([
		{
			title: m.landing_footer_column_product(),
			links: [
				{ label: m.landing_footer_link_features(), href: '#features' },
				{ label: m.landing_footer_link_dashboard(), href: dashboardHref('/') }
			]
		},
		{
			title: m.landing_footer_column_developers(),
			links: [
				{ label: m.landing_footer_link_documentation(), href: resolve('/docs') },
				{ label: m.landing_footer_link_sdks(), href: resolve('/docs') },
				{ label: m.landing_footer_link_sign_in(), href: dashboardHref('/login') }
			]
		},
		{
			title: m.landing_footer_column_legal(),
			links: [
				{ label: m.landing_footer_link_privacy(), href: dashboardHref('/privacy') },
				{ label: m.landing_footer_link_terms(), href: dashboardHref('/terms') }
			]
		}
	]);

	const year = new Date().getFullYear();
</script>

<!-- Footer hrefs are static hash anchors (in-page) or routes pre-resolved above. -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->
<footer class="border-t border-border/60 bg-muted/20">
	<div class="mx-auto max-w-7xl px-4 py-14 sm:px-6">
		<div class="grid gap-10 sm:grid-cols-2 lg:grid-cols-4">
			<div class="lg:col-span-1">
				<a href={resolve('/')} class="flex items-center gap-2 font-semibold tracking-tight">
					<span
						class="flex size-8 items-center justify-center rounded-lg bg-gradient-to-br from-brand to-brand-2 text-brand-foreground"
					>
						<ShieldCheckIcon class="size-5" />
					</span>
					<span class="text-lg">GateKeeper</span>
				</a>
				<p class="mt-4 max-w-xs text-sm text-muted-foreground">
					{m.landing_footer_tagline()}
				</p>
			</div>

			{#each columns as column (column.title)}
				<div>
					<h3 class="text-sm font-semibold">{column.title}</h3>
					<ul class="mt-4 space-y-3">
						{#each column.links as link (link.label)}
							<li>
								<a
									href={link.href}
									class="text-sm text-muted-foreground transition-colors hover:text-foreground"
								>
									{link.label}
								</a>
							</li>
						{/each}
					</ul>
				</div>
			{/each}
		</div>

		<div
			class="mt-12 flex flex-col items-center justify-between gap-4 border-t border-border/60 pt-8 sm:flex-row"
		>
			<p class="text-sm text-muted-foreground">{m.landing_footer_copyright({ year })}</p>
			<div class="flex items-center gap-2"><LanguageSwitcher /><ThemeToggle /></div>
		</div>
	</div>
</footer>
