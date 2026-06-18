<script lang="ts">
	import LegalDocumentView from '$lib/components/legal/legal-document-view.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>Privacy Policy - GateKeeper</title>
	<meta name="description" content="How GateKeeper collects, uses, and protects your data." />
</svelte:head>

<LegalDocumentView title="Privacy Policy" content={data.content} updatedAt={data.updatedAt} />
