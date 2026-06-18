<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteTenantGroup, type TenantGroupView } from '$lib/services/admin/groups';
	import GroupAvatar from './group-avatar.svelte';

	type Props = {
		tenantId: string;
		group: TenantGroupView | null;
		onOpenChange: (open: boolean) => void;
		onDeleted?: () => void;
	};

	let { tenantId, group, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	async function confirmDelete() {
		if (!group) return;

		deleting = true;
		const [err] = await deleteTenantGroup(tenantId, group.group_id);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success('Group deleted');
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
		onOpenChange(false);
		onDeleted?.();
	}
</script>

<Dialog.Root open={!!group} onOpenChange={(open) => !open && onOpenChange(false)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="mb-1 flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>Delete this group?</Dialog.Title>
			<Dialog.Description>
				This removes the group and all its memberships. This action can't be undone.
			</Dialog.Description>
		</Dialog.Header>

		{#if group}
			<div class="bg-muted/40 flex items-center gap-3 rounded-lg border p-3">
				<GroupAvatar {group} />
				<div class="min-w-0">
					<p class="truncate text-sm font-medium">{group.name}</p>
					{#if group.description}
						<p class="text-muted-foreground truncate text-xs">{group.description}</p>
					{/if}
				</div>
			</div>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => onOpenChange(false)}>
				Cancel
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				Delete group
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
