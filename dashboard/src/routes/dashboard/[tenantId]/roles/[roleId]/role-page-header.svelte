<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';

	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';

	import { Button } from '$lib/components/ui/button/index.js';
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
	let idCopied = $state(false);

	async function copyId() {
		try {
			await navigator.clipboard.writeText(role.role_id);
			idCopied = true;
			setTimeout(() => (idCopied = false), 1500);
			toast.success('Role ID copied to clipboard');
		} catch {
			toast.error('Could not copy to clipboard');
		}
	}
</script>

<div class="flex flex-wrap items-start justify-between gap-4">
	<div class="flex items-center gap-4">
		<RoleAvatar {role} class="size-14 text-lg" />
		<div class="min-w-0">
			<h1 class="truncate text-3xl font-bold tracking-tight">{role.name}</h1>
			{#if role.description}
				<button
					type="button"
					onclick={copyId}
					class="text-muted-foreground hover:text-foreground group mt-1 flex items-center gap-1.5 text-sm"
				>
					<span class="truncate">{role.description}</span>
				</button>
			{:else}
				<button
					type="button"
					onclick={copyId}
					class="text-muted-foreground hover:text-foreground group mt-1 flex items-center gap-1.5 text-sm"
				>
					<span class="font-mono text-xs">{role.role_id}</span>
					{#if idCopied}
						<CheckIcon class="size-3.5 text-emerald-600 dark:text-emerald-400" />
					{:else}
						<CopyIcon class="size-3.5 opacity-0 transition group-hover:opacity-100" />
					{/if}
				</button>
			{/if}
		</div>
	</div>

	<div class="flex gap-2">
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
