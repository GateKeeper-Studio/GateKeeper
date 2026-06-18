<script lang="ts">
	import LegalDocumentView from '$lib/components/legal/legal-document-view.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>Terms of Service - GateKeeper</title>
	<meta name="description" content="The terms that govern your use of the service." />
</svelte:head>

<LegalDocumentView title="Terms of Service" content={data.content} updatedAt={data.updatedAt} />
