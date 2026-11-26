<script lang="ts">
	import Background from '$lib/components/auth/background.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import ApprovePanel from './approve-panel.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{m.qr_approve_meta_title()} - GateKeeper</title>
	<!-- The code's secret is in this page's address: never let it leave as a referrer. -->
	<meta name="referrer" content="no-referrer" />
</svelte:head>

<Background client={data.client} termsAndConditionsEnabled={false}>
	{#if data.loadError}
		<ErrorAlert message={data.loadError.message} title={m.auth_error_title()} />
	{:else}
		<ApprovePanel />
	{/if}
</Background>
