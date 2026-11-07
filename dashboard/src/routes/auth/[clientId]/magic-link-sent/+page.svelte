<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';

	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';
	import { m } from '$lib/paraglide/messages.js';

	let { data }: { data: PageData } = $props();

	const backHref = $derived(
		resolve(`/auth/[clientId]/sign-in?${page.url.searchParams.toString()}`, {
			clientId: data.clientId
		})
	);
</script>

<svelte:head>
	<title>{m.magic_link_sent_meta_title()} - GateKeeper</title>
</svelte:head>

<Background client={data.client} termsAndConditionsEnabled={false}>
	<a href={backHref} class="mb-6 flex items-center gap-3 text-primary hover:underline">
		<div
			class="flex items-center justify-center rounded-md bg-secondary p-2 text-primary transition-all hover:brightness-90"
		>
			<ArrowLeftIcon />
		</div>
		{m.magic_link_sent_back()}
	</a>

	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">{m.magic_link_sent_heading()}</h1>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else}
		<AuthForm />
	{/if}
</Background>
