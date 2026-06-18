<script lang="ts">
	import { Markdown } from 'carta-md';

	import { carta } from '$lib/carta';
	import LegalPage from './legal-page.svelte';

	let {
		title,
		content,
		updatedAt
	}: {
		title: string;
		content: string;
		updatedAt: string | null;
	} = $props();

	const lastUpdated = $derived(
		updatedAt
			? new Date(updatedAt).toLocaleDateString(undefined, {
					year: 'numeric',
					month: 'long',
					day: 'numeric'
				})
			: '—'
	);
</script>

<LegalPage {title} {lastUpdated}>
	{#key content}
		<Markdown {carta} value={content} />
	{/key}
</LegalPage>
