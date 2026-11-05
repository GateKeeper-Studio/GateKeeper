<script lang="ts">
	import { goto } from '$app/navigation';

	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import CopyIdButton from '$lib/components/copy-id-button.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import type { TenantGroupView } from '$lib/services/admin/groups';

	import GroupAvatar from '../group-avatar.svelte';
	import DeleteGroupDialog from '../delete-group-dialog.svelte';

	type Props = {
		group: TenantGroupView;
		tenantId: string;
		groupId: string;
		listHref: string;
	};

	let { group, tenantId, groupId, listHref }: Props = $props();

	let deleteTarget = $state<TenantGroupView | null>(null);
</script>

<div class="flex flex-wrap items-start justify-between gap-4">
	<div class="flex items-center gap-4">
		<GroupAvatar {group} class="size-14 text-lg" />
		<div class="min-w-0">
			<h1 class="truncate text-3xl font-bold tracking-tight">{group.name}</h1>
			{#if group.description}
				<p class="text-muted-foreground mt-1 truncate text-sm">{group.description}</p>
			{:else}
				<p class="text-muted-foreground mt-1 truncate font-mono text-xs">{group.group_id}</p>
			{/if}
		</div>
	</div>

	<div class="flex gap-2">
		<CopyIdButton id={groupId} />
		<Button variant="outline" href={`${listHref}/${groupId}/edit-group`}>
			<PencilIcon /> {m.groups_edit_label()}
		</Button>
		<Button variant="destructive" onclick={() => (deleteTarget = group)}>
			<Trash2Icon /> {m.groups_delete_label()}
		</Button>
	</div>
</div>

<DeleteGroupDialog
	{tenantId}
	group={deleteTarget}
	onOpenChange={() => (deleteTarget = null)}
	onDeleted={() => goto(listHref)}
/>
