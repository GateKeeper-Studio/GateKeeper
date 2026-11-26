<script lang="ts">
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';
	import DownloadIcon from '@lucide/svelte/icons/download';

	import { Button } from '$lib/components/ui/button/index.js';
	import { m } from '$lib/paraglide/messages.js';

	// Recovery codes as the server returned them, shown once. The server keeps only
	// their hashes, so this is the admin's single chance to save them.
	let { codes }: { codes: string[] } = $props();

	let copied = $state(false);
	const text = $derived(codes.join('\n') + '\n');

	async function copy() {
		await navigator.clipboard.writeText(text);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	function download() {
		const url = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = 'gatekeeper-recovery-codes.txt';
		link.click();
		URL.revokeObjectURL(url);
	}
</script>

<div class="space-y-3">
	<ul class="grid grid-cols-2 gap-2 rounded-lg border bg-muted/40 p-4 font-mono text-sm">
		{#each codes as code (code)}
			<li class="tracking-wider">{code}</li>
		{/each}
	</ul>
	<div class="flex gap-2">
		<Button type="button" variant="outline" size="sm" onclick={copy}>
			{#if copied}<CheckIcon />{:else}<CopyIcon />{/if}
			{copied ? m.settings_mfa_copied() : m.settings_mfa_copy_codes()}
		</Button>
		<Button type="button" variant="outline" size="sm" onclick={download}>
			<DownloadIcon />
			{m.settings_mfa_download_codes()}
		</Button>
	</div>
</div>
