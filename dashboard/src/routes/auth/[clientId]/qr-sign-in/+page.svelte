<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import QrPanel from './qr-panel.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{m.qr_sign_in_meta_title()} - GateKeeper</title>
</svelte:head>

<Background client={data.client}>
	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">{m.qr_sign_in_heading()}</h1>
		<p class="text-sm text-muted-foreground">{m.qr_sign_in_description()}</p>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else if data.client && !data.client.qrLoginEnabled}
		<ErrorAlert message={m.qr_sign_in_unavailable()} title={m.auth_error_title()} />
	{:else}
		<QrPanel client={data.client} />
	{/if}
</Background>
