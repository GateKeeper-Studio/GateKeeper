<script lang="ts">
	import { toast } from 'svelte-sonner';

	import MoreHorizontalIcon from '@lucide/svelte/icons/ellipsis';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';

	import type { TenantUserView } from '$lib/services/admin/users';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		user: TenantUserView;
		tenantId: string;
		onDelete: (user: TenantUserView) => void;
	};

	let { user, tenantId, onDelete }: Props = $props();

	async function copyId() {
		try {
			await navigator.clipboard.writeText(user.user_id);
			toast.success(m.users_copy_id_success());
		} catch {
			toast.error(m.users_copy_clipboard_error());
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
					<span class="sr-only">{m.users_actions_open_menu()}</span>
					<MoreHorizontalIcon />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>

		<DropdownMenu.Content align="end">
			<DropdownMenu.Label>{m.users_action_actions_label()}</DropdownMenu.Label>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/users/${user.user_id}`} {...props}>
						<EyeIcon class="size-4" />
						{m.users_action_view_user()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={copyId}>
				<CopyIcon class="size-4" />
				{m.users_action_copy_user_id()}
			</DropdownMenu.Item>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/tenants/${tenantId}/users/${user.user_id}/edit-user`} {...props}>
						<PencilIcon class="size-4" />
						{m.users_action_edit_user()}
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item variant="destructive" onSelect={() => onDelete(user)}>
				<Trash2Icon class="size-4" />
				{m.users_delete_user_button()}
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
