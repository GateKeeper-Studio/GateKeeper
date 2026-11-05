<script lang="ts">
	import { toast } from 'svelte-sonner';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';

	import { Button } from '$lib/components/ui/button/index.js';

	// A small "Copy ID" button for entity detail headers (users, groups, roles,
	// OAuth clients, …). Copies the entity's identifier and briefly shows a check.
	type Props = {
		id: string;
		label?: string;
		size?: 'default' | 'sm' | 'lg' | 'icon';
	};
	let { id, label = 'Copy ID', size = 'default' }: Props = $props();

	let copied = $state(false);

	async function copy() {
		try {
			await navigator.clipboard.writeText(id);
			copied = true;
			setTimeout(() => (copied = false), 1500);
			toast.success('ID copied to clipboard');
		} catch {
			toast.error('Could not copy to clipboard');
		}
	}
</script>

<Button variant="outline" {size} onclick={copy} title={id}>
	{#if copied}
		<CheckIcon class="text-emerald-600 dark:text-emerald-400" />
	{:else}
		<CopyIcon />
	{/if}
	{label}
</Button>
