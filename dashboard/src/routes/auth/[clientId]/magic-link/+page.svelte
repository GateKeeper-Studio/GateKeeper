<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';
	import { m } from '$lib/paraglide/messages.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{m.magic_link_meta_title()} - GateKeeper</title>
	<!-- The link's secret is in the fragment, which is never sent anyway; this keeps the
	     page itself from leaking its URL to anything it loads or links to. -->
	<meta name="referrer" content="no-referrer" />
</svelte:head>

<Background client={data.client} termsAndConditionsEnabled={false}>
	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">{m.magic_link_heading()}</h1>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else}
		<AuthForm />
	{/if}
</Background>
