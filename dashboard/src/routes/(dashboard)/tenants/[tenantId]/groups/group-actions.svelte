<script lang="ts">
	import { toast } from 'svelte-sonner';

	import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	import { m } from '$lib/paraglide/messages.js';
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
			toast.success(m.groups_copy_id_success_toast());
		} catch {
			toast.error(m.groups_copy_id_error_toast());
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
					<span class="sr-only">{m.groups_open_menu_sr()}</span>
					<MoreHorizontalIcon />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>

		<DropdownMenu.Content align="end">
			<DropdownMenu.Label>{m.groups_actions_label()}</DropdownMenu.Label>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/groups/${group.group_id}`} {...props}>
						<EyeIcon class="size-4" />
						{m.groups_view_group()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={copyId}>
				<CopyIcon class="size-4" />
				{m.groups_copy_group_id()}
			</DropdownMenu.Item>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/groups/${group.group_id}/edit-group`} {...props}>
						<PencilIcon class="size-4" />
						{m.groups_edit_group()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item variant="destructive" onSelect={() => onDelete(group)}>
				<Trash2Icon class="size-4" />
				{m.groups_delete_button()}
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
