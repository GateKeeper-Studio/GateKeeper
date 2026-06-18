<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';

	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import ResendAlert from './resend-alert.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const tryingToSignIn = $derived(page.url.searchParams.get('trying_to_sign_in') === 'true');
	const backHref = $derived(
		resolve(`/auth/[clientId]/sign-in?${page.url.searchParams.toString()}`, {
			clientId: data.clientId
		})
	);
</script>

<svelte:head>
	<title>Confirm E-mail - GateKeeper</title>
</svelte:head>

<Background client={data.client}>
	<a
		href={backHref}
		class="absolute top-4 left-4 flex items-center gap-3 text-primary hover:underline"
	>
		<div
			class="flex items-center justify-center rounded-md bg-secondary p-2 text-primary transition-all hover:brightness-90"
		>
			<ArrowLeftIcon />
		</div>
		Back to sign in
	</a>

	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">Confirm E-mail</h1>

		{#if tryingToSignIn}
			<ResendAlert />
		{:else}
			<p class="text-sm text-muted-foreground">
				We sent a confirmation code to your e-mail. Enter the code below to confirm your e-mail
			</p>
		{/if}
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title="An error occurred..." />
	{:else}
		<AuthForm />
	{/if}
</Background>
