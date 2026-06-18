<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import ShieldOffIcon from '@lucide/svelte/icons/shield-off';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteOAuthClient, type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { clientBadges } from '../display.js';

	type Props = {
		client: OAuthClientView;
		tenantId: string;
		clientId: string;
		listHref: string;
	};

	let { client, tenantId, clientId, listHref }: Props = $props();

	const queryClient = useQueryClient();
	let deleteOpen = $state(false);
	let deleting = $state(false);

	async function confirmDelete() {
		deleting = true;
		const [err] = await deleteOAuthClient(tenantId, clientId);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success('OAuth client deleted');
		deleteOpen = false;
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });
		await goto(listHref);
	}
</script>

<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
	<div class="min-w-0">
		<div class="flex flex-wrap items-center gap-2">
			<h1 class="truncate text-2xl font-semibold tracking-tight">{client.name}</h1>
			{#if client.is_active}
				<Badge variant="default" class="gap-1"><ShieldCheckIcon /> Active</Badge>
			{:else}
				<Badge variant="secondary" class="gap-1"><ShieldOffIcon /> Inactive</Badge>
			{/if}
		</div>
		<p class="mt-1 truncate font-mono text-xs text-muted-foreground">{client.client_id}</p>

		<div class="mt-3 flex flex-wrap gap-1.5">
			{#each clientBadges(client) as badge (badge.label)}
				<Badge variant={badge.variant}>{badge.label}</Badge>
			{/each}
		</div>
	</div>

	<div class="flex gap-2">
		<Button variant="outline" size="sm" href={`${listHref}/${clientId}/edit`}>
			<PencilIcon /> Edit
		</Button>
		<Button variant="destructive" size="sm" onclick={() => (deleteOpen = true)}>
			<Trash2Icon /> Delete
		</Button>
	</div>
</div>

<Dialog.Root bind:open={deleteOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Delete "{client.name}"?</Dialog.Title>
			<Dialog.Description>
				This permanently removes the client. Applications using it will no longer be able to sign
				users in. This cannot be undone.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => (deleteOpen = false)}>
				Cancel
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				Delete client
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
