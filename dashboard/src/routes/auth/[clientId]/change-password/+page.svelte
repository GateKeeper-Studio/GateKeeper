<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';
	import { m } from '$lib/paraglide/messages.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{m.auth_change_password_meta_title()} - GateKeeper</title>
</svelte:head>

<Background client={data.client} termsAndConditionsEnabled={false}>
	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">{m.auth_change_password_heading()}</h1>

		<p class="text-sm text-muted-foreground">{m.auth_change_password_description()}</p>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else}
		<AuthForm />
	{/if}
</Background>
