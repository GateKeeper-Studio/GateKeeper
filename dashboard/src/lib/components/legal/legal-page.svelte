<script lang="ts">
	import type { Snippet } from 'svelte';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import LanguageSwitcher from '$lib/components/language-switcher.svelte';
	import { websiteHref } from '$lib/external-urls';

	let {
		title,
		lastUpdated,
		children
	}: {
		title: string;
		lastUpdated: string;
		children: Snippet;
	} = $props();

	function goBack() {
		if (typeof window !== 'undefined' && window.history.length > 1) {
			window.history.back();
		} else {
			window.location.assign(websiteHref('/'));
		}
	}
</script>

<div class="flex min-h-screen flex-col bg-muted/40">
	<header class="sticky top-0 z-10 border-b bg-background/80 backdrop-blur">
		<div class="mx-auto flex w-full max-w-3xl items-center justify-between gap-4 px-6 py-4">
			<Button variant="ghost" size="sm" class="gap-2" onclick={goBack}>
				<ArrowLeftIcon class="size-4" />
				Back
			</Button>

			<div class="flex items-center gap-2"><LanguageSwitcher /><ThemeToggle /></div>
		</div>
	</header>

	<main class="mx-auto w-full max-w-3xl flex-1 px-6 py-10">
		<Card.Root>
			<Card.Header>
				<Card.Title class="text-3xl font-semibold tracking-tight">{title}</Card.Title>
				<Card.Description>Last updated: {lastUpdated}</Card.Description>
			</Card.Header>

			<Separator />

			<Card.Content class="pt-6">
				<div
					class="prose max-w-none prose-neutral dark:prose-invert prose-headings:font-semibold prose-a:text-primary"
				>
					{@render children()}
				</div>
			</Card.Content>
		</Card.Root>
	</main>
</div>
