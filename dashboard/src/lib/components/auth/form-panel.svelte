<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
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

	// Prefer the client's own external legal URLs; otherwise link to this tenant's
	// hosted pages (resolved by client id), falling back to the platform default
	// pages when there is no client context.
	const termsHref = $derived(
		client?.tosUri ??
			(client ? resolve('/terms/[clientId]', { clientId: client.id }) : resolve('/terms'))
	);
	const privacyHref = $derived(
		client?.policyUri ??
			(client ? resolve('/privacy/[clientId]', { clientId: client.id }) : resolve('/privacy'))
	);
</script>

<div class="mx-auto flex w-full flex-col justify-center space-y-6 sm:w-[350px]">
	{#if appearance.logoUri}
		<!-- Admin-supplied logo. The URL is validated http(s) server-side and rendered
		     as an <img> src, so it can never inject markup or a javascript: URL. -->
		<img src={appearance.logoUri} alt="" class="mx-auto h-10 w-auto object-contain" />
	{/if}

	{@render children()}

	{#if termsAndConditionsEnabled}
		<p class="px-8 text-center text-sm text-muted-foreground">
			By clicking continue, you agree to our
			<!-- Client-provided tosUri may be external; fallback is already resolved above. -->
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={termsHref} class="mx-1 underline underline-offset-4 hover:text-primary">
				Terms of Service
			</a>
			and
			<!-- Client-provided policyUri may be external; fallback is already resolved above. -->
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={privacyHref} class="ml-1 underline underline-offset-4 hover:text-primary">
				Privacy Policy
			</a>
			.
		</p>
	{/if}
</div>
