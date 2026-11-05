<script lang="ts">
	import { goto } from '$app/navigation';

	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import CopyIdButton from '$lib/components/copy-id-button.svelte';
	import type { TenantRoleView } from '$lib/services/admin/roles';

	import RoleAvatar from '../role-avatar.svelte';
	import DeleteRoleDialog from '../delete-role-dialog.svelte';

	type Props = {
		role: TenantRoleView;
		tenantId: string;
		roleId: string;
		listHref: string;
	};

	let { role, tenantId, roleId, listHref }: Props = $props();

	let deleteTarget = $state<TenantRoleView | null>(null);
</script>

<div class="flex flex-wrap items-start justify-between gap-4">
	<div class="flex items-center gap-4">
		<RoleAvatar {role} class="size-14 text-lg" />
		<div class="min-w-0">
			<h1 class="truncate text-3xl font-bold tracking-tight">{role.name}</h1>
			{#if role.description}
				<p class="text-muted-foreground mt-1 truncate text-sm">{role.description}</p>
			{:else}
				<p class="text-muted-foreground mt-1 truncate font-mono text-xs">{role.role_id}</p>
			{/if}
		</div>
	</div>

	<div class="flex gap-2">
		<CopyIdButton id={roleId} />
		<Button variant="outline" href={`${listHref}/${roleId}/edit-role`}>
			<PencilIcon /> Edit
		</Button>
		<Button variant="destructive" onclick={() => (deleteTarget = role)}>
			<Trash2Icon /> Delete
		</Button>
	</div>
</div>

<DeleteRoleDialog
	{tenantId}
	role={deleteTarget}
	onOpenChange={() => (deleteTarget = null)}
	onDeleted={() => goto(listHref)}
/>
