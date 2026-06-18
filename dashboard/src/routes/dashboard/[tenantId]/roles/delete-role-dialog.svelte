<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteTenantRole, type TenantRoleView } from '$lib/services/admin/roles';
	import RoleAvatar from './role-avatar.svelte';

	type Props = {
		tenantId: string;
		role: TenantRoleView | null;
		onOpenChange: (open: boolean) => void;
		onDeleted?: () => void;
	};

	let { tenantId, role, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	async function confirmDelete() {
		if (!role) return;

		deleting = true;
		const [err] = await deleteTenantRole(tenantId, role.role_id);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success('Role deleted');
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });
		onOpenChange(false);
		onDeleted?.();
	}
</script>

<Dialog.Root open={!!role} onOpenChange={(open) => !open && onOpenChange(false)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="bg-destructive/10 text-destructive mb-1 flex size-11 items-center justify-center rounded-full"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>Delete this role?</Dialog.Title>
			<Dialog.Description>
				This removes the role and all its assignments. This action can't be undone.
			</Dialog.Description>
		</Dialog.Header>

		{#if role}
			<div class="bg-muted/40 flex items-center gap-3 rounded-lg border p-3">
				<RoleAvatar {role} />
				<div class="min-w-0">
					<p class="truncate text-sm font-medium">{role.name}</p>
					{#if role.description}
						<p class="text-muted-foreground truncate text-xs">{role.description}</p>
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
				Delete role
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
