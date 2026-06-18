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

	type Props = {
		user: TenantUserView;
		tenantId: string;
		onDelete: (user: TenantUserView) => void;
	};

	let { user, tenantId, onDelete }: Props = $props();

	async function copyId() {
		try {
			await navigator.clipboard.writeText(user.user_id);
			toast.success('User ID copied to clipboard');
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
					<a href={`/dashboard/${tenantId}/users/${user.user_id}`} {...props}>
						<EyeIcon class="size-4" />
						View user
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Item onSelect={copyId}>
				<CopyIcon class="size-4" />
				Copy user ID
			</DropdownMenu.Item>
			<DropdownMenu.Item>
				{#snippet child({ props })}
					<a href={`/dashboard/${tenantId}/users/${user.user_id}/edit-user`} {...props}>
						<PencilIcon class="size-4" />
						Edit user
					</a>
				{/snippet}
			</DropdownMenu.Item>
			<DropdownMenu.Separator />
			<DropdownMenu.Item variant="destructive" onSelect={() => onDelete(user)}>
				<Trash2Icon class="size-4" />
				Delete user
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</div>
