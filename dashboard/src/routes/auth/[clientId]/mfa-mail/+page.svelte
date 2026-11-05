<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import MfaMethodHeader from '$lib/components/auth/mfa-method-header.svelte';
	import { EMfaType } from '$lib/services/auth/login';
	import AuthForm from './auth-form.svelte';
	import type { PageData } from './$types';
	import { m } from '$lib/paraglide/messages.js';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{m.auth_mfa_mail_title()} - GateKeeper</title>
</svelte:head>

<Background client={data.client}>
	<MfaMethodHeader method={EMfaType.MfaEmail} />

	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else}
		<AuthForm client={data.client} />
	{/if}
</Background>
