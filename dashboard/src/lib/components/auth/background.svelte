<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import type { ClientAuthData } from '$lib/services/auth/get-client-auth-data';

	let {
		client,
		termsAndConditionsEnabled = true,
		children
	}: {
		client: ClientAuthData | null;
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

<div
	class="relative flex h-screen flex-col items-center justify-center lg:grid lg:max-w-none lg:grid-cols-2 lg:px-0"
>
	<div class="relative hidden h-screen flex-col bg-muted p-10 text-white lg:flex dark:border-r">
		<div
			class="absolute inset-0 bg-cover"
			style="background-image: url(https://images.unsplash.com/photo-1590069261209-f8e9b8642343?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1376&q=80)"
		></div>

		<div class="relative z-20 flex items-center text-lg font-medium">
			{client?.name}
		</div>

		<div class="relative z-20 mt-auto">
			<blockquote class="space-y-2">
				<p class="text-lg">
					&ldquo;This library has saved me countless hours of work and helped me deliver stunning
					designs to my clients faster than ever before. Highly recommended!&rdquo;
				</p>

				<footer class="text-sm">Sofia Davis</footer>
			</blockquote>
		</div>
	</div>

	<div class="relative flex h-full w-full flex-1 items-center justify-center overflow-auto">
		<div class="flex w-full justify-center p-6 sm:p-8">
			<span class="absolute right-4 bottom-4">
				<ThemeToggle />
			</span>

			<div class="mx-auto flex w-full flex-col justify-center space-y-6 sm:w-[350px]">
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
		</div>
	</div>
</div>
