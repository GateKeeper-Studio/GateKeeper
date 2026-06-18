<script lang="ts">
	import { toast } from 'svelte-sonner';

	import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	import type { TenantGroupView } from '$lib/services/admin/groups';

	type Props = {
		group: TenantGroupView;
		tenantId: string;
		onDelete: (group: TenantGroupView) => void;
	};

	let { group, tenantId, onDelete }: Props = $props();

	async function copyId() {
		try {
			await navigator.clipboard.writeText(group.group_id);
			toast.success('Group ID copied to clipboard');
		} catch {
			toast.error('Could not copy to clipboard');
		}
	}
</script>

<div class="text-right">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="ghost"
					size="icon"
					class="text-muted-foreground hover:text-foreground size-8"
				>
					<span class="sr-only">Open menu</span>
					<MoreHorizontalIcon />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>

		<DropdownMenu.Content align="end">
			<DropdownMenu.Label>Actions</DropdownMenu.Label>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/dashboard/${tenantId}/groups/${group.group_id}`} {...props}>
						<EyeIcon class="size-4" />
						View group
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={copyId}>
				<CopyIcon class="size-4" />
				Copy group ID
			</DropdownMenu.Item>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/dashboard/${tenantId}/groups/${group.group_id}/edit-group`} {...props}>
						<PencilIcon class="size-4" />
						Edit group
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item variant="destructive" onSelect={() => onDelete(group)}>
				<Trash2Icon class="size-4" />
				Delete group
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
