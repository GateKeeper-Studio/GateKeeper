<script lang="ts">
	import { toast } from 'svelte-sonner';

	import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	import type { TenantRoleView } from '$lib/services/admin/roles';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		role: TenantRoleView;
		tenantId: string;
		onDelete: (role: TenantRoleView) => void;
	};

	let { role, tenantId, onDelete }: Props = $props();

	async function copyId() {
		try {
			await navigator.clipboard.writeText(role.role_id);
			toast.success(m.roles_copy_id_success());
		} catch {
			toast.error(m.roles_copy_clipboard_error());
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
					<span class="sr-only">{m.roles_actions_open_menu()}</span>
					<MoreHorizontalIcon />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>

		<DropdownMenu.Content align="end">
			<DropdownMenu.Label>{m.roles_action_actions_label()}</DropdownMenu.Label>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/roles/${role.role_id}`} {...props}>
						<EyeIcon class="size-4" />
						{m.roles_action_view_role()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={copyId}>
				<CopyIcon class="size-4" />
				{m.roles_action_copy_role_id()}
			</DropdownMenu.Item>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/roles/${role.role_id}/edit-role`} {...props}>
						<PencilIcon class="size-4" />
						{m.roles_edit_role_title()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item variant="destructive" onSelect={() => onDelete(role)}>
				<Trash2Icon class="size-4" />
				{m.roles_delete_role_button()}
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
