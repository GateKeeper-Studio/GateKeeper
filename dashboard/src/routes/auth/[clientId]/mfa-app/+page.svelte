<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MfaMethodHeader from '$lib/components/auth/mfa-method-header.svelte';
	import { EMfaType } from '$lib/services/auth/login';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>Authenticator App - GateKeeper</title>
</svelte:head>

<Background client={data.client}>
	<MfaMethodHeader method={EMfaType.MfaApp} />

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title="An error occurred..." />
	{:else}
		<AuthForm />
	{/if}
</Background>
