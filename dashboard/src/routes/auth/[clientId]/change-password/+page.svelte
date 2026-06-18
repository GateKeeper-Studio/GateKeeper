<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Change Password - GateKeeper</title>
</svelte:head>

<Background client={data.client} termsAndConditionsEnabled={false}>
	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">Change your password</h1>

		<p class="text-sm text-muted-foreground">Enter your new password below to continue</p>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title="An error occurred..." />
	{:else}
		<AuthForm />
	{/if}
</Background>
