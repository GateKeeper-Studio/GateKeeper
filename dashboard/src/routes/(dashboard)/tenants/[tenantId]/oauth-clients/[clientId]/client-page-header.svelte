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
	import CopyIdButton from '$lib/components/copy-id-button.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteOAuthClient, type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { m } from '$lib/paraglide/messages.js';
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

		toast.success(m.oauth_clients_toast_client_deleted());
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
				<Badge variant="default" class="gap-1"
					><ShieldCheckIcon /> {m.oauth_clients_status_active()}</Badge
				>
			{:else}
				<Badge variant="secondary" class="gap-1"
					><ShieldOffIcon /> {m.oauth_clients_status_inactive()}</Badge
				>
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
		<CopyIdButton id={client.client_id} size="sm" />
		<Button variant="outline" size="sm" href={`${listHref}/${clientId}/edit`}>
			<PencilIcon /> {m.oauth_clients_action_edit()}
		</Button>
		<Button variant="destructive" size="sm" onclick={() => (deleteOpen = true)}>
			<Trash2Icon /> {m.oauth_clients_action_delete()}
		</Button>
	</div>
</div>

<Dialog.Root bind:open={deleteOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>{m.oauth_clients_delete_dialog_confirm_message({ name: client.name })}</Dialog.Title>
			<Dialog.Description>
				{m.oauth_clients_delete_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => (deleteOpen = false)}>
				{m.oauth_clients_action_cancel()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.oauth_clients_delete_dialog_confirm_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
