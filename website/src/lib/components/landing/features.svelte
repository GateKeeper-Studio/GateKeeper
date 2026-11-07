<script lang="ts">
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import Building2Icon from '@lucide/svelte/icons/building-2';
	import FingerprintIcon from '@lucide/svelte/icons/fingerprint';
	import BlocksIcon from '@lucide/svelte/icons/blocks';
	import UsersIcon from '@lucide/svelte/icons/users';
	import LockIcon from '@lucide/svelte/icons/lock';
	import CpuIcon from '@lucide/svelte/icons/cpu';
	import PaletteIcon from '@lucide/svelte/icons/palette';
	import type { Component } from 'svelte';

	import * as Card from '$lib/components/ui/card/index.js';
	import { reveal } from '$lib/actions/reveal';
	import ImagePlaceholder from './image-placeholder.svelte';
	import { m } from '$lib/paraglide/messages.js';

	type Feature = { icon: Component; title: string; description: string };

	const features: Feature[] = $derived([
		{
			icon: KeyRoundIcon,
			title: m.landing_features_oauth_title(),
			description: m.landing_features_oauth_description()
		},
		{
			icon: Building2Icon,
			title: m.landing_features_multi_tenant_title(),
			description: m.landing_features_multi_tenant_description()
		},
		{
			icon: FingerprintIcon,
			title: m.landing_features_mfa_title(),
			description: m.landing_features_mfa_description()
		},
		{
			icon: BlocksIcon,
			title: m.landing_features_sdks_title(),
			description: m.landing_features_sdks_description()
		},
		{
			icon: UsersIcon,
			title: m.landing_features_user_mgmt_title(),
			description: m.landing_features_user_mgmt_description()
		},
		{
			icon: CpuIcon,
			title: m.landing_features_rust_core_title(),
			description: m.landing_features_rust_core_description()
		},
		{
			icon: LockIcon,
			title: m.landing_features_secure_default_title(),
			description: m.landing_features_secure_default_description()
		}
	]);
</script>

<section id="features" class="mx-auto max-w-7xl scroll-mt-20 px-4 py-24 sm:px-6">
	<div class="mx-auto max-w-2xl text-center" use:reveal>
		<span class="text-sm font-semibold tracking-wide text-brand uppercase"
			>{m.landing_features_eyebrow()}</span
		>
		<h2 class="mt-3 text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
			{m.landing_features_heading()}
		</h2>
		<p class="mt-4 text-lg text-pretty text-muted-foreground">
			{m.landing_features_subheading()}
		</p>
	</div>

	<div class="mt-14 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
		<!-- Large showcase cell -->
		<div class="sm:col-span-2 lg:row-span-2" use:reveal={{ threshold: 0.1 }}>
			<Card.Root class="group relative h-full overflow-hidden">
				<Card.Header>
					<div
						class="mb-2 flex size-11 items-center justify-center rounded-xl bg-gradient-to-br from-brand to-brand-2 text-brand-foreground"
					>
						<PaletteIcon class="size-5" />
					</div>
					<Card.Title class="text-xl">{m.landing_features_showcase_title()}</Card.Title>
					<Card.Description class="text-base">
						{m.landing_features_showcase_description()}
					</Card.Description>
				</Card.Header>
				<Card.Content>
					<ImagePlaceholder label={m.landing_features_showcase_image_label()} icon={PaletteIcon} />
				</Card.Content>
			</Card.Root>
		</div>

		{#each features as feature, i (feature.title)}
			{@const Icon = feature.icon}
			<div use:reveal={{ delay: 60 * (i + 1), threshold: 0.1 }}>
				<Card.Root class="h-full transition-colors hover:border-brand/40">
					<Card.Header>
						<div
							class="mb-2 flex size-10 items-center justify-center rounded-lg border border-border bg-muted text-brand"
						>
							<Icon class="size-5" />
						</div>
						<Card.Title>{feature.title}</Card.Title>
						<Card.Description>{feature.description}</Card.Description>
					</Card.Header>
				</Card.Root>
			</div>
		{/each}
	</div>
</section>
