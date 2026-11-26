<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteTenantUser, type TenantUserView } from '$lib/services/admin/users';
	import UserAvatar from './user-avatar.svelte';
	import { userFullName } from './display.js';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		// The user pending deletion; null keeps the dialog closed.
		user: TenantUserView | null;
		onOpenChange: (open: boolean) => void;
		// Optional hook fired after a successful delete (e.g. to navigate away).
		onDeleted?: () => void;
	};

	let { tenantId, user, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	async function confirmDelete() {
		if (!user) return;

		deleting = true;
		const [err] = await deleteTenantUser(tenantId, user.user_id);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success(m.users_delete_user_success());
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId] });
		onOpenChange(false);
		onDeleted?.();
	}
</script>

<Dialog.Root open={!!user} onOpenChange={(open) => !open && onOpenChange(false)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="mb-1 flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>{m.users_delete_dialog_title()}</Dialog.Title>
			<Dialog.Description>
				{m.users_delete_dialog_description()}
			</Dialog.Description>
		</Dialog.Header>

		{#if user}
			<div class="bg-muted/40 flex items-center gap-3 rounded-lg border p-3">
				<UserAvatar {user} />
				<div class="min-w-0">
					<p class="truncate text-sm font-medium">{userFullName(user)}</p>
					<p class="text-muted-foreground truncate text-xs">{user.email}</p>
				</div>
			</div>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => onOpenChange(false)}>
				{m.users_action_cancel()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.users_delete_user_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
