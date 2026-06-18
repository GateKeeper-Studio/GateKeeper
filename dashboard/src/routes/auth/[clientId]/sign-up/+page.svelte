<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Sign Up - GateKeeper</title>
	<meta name="description" content="Sign up for an account" />
</svelte:head>

<Background client={data.client}>
	<div class="flex flex-col space-y-2 text-center">
		<h1 class="text-2xl font-semibold tracking-tight">Create an account</h1>

		<p class="text-sm text-muted-foreground">Enter your data below to create your account</p>
	</div>

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title="An error occurred..." />
	{:else}
		<AuthForm />
	{/if}
</Background>
