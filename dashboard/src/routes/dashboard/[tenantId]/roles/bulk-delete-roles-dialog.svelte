<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteTenantRole, type TenantRoleView } from '$lib/services/admin/roles';

	type Props = {
		tenantId: string;
		roleIds: string[];
		sample: TenantRoleView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onDeleted: () => void;
	};

	let { tenantId, roleIds, sample, open, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	const count = $derived(roleIds.length);
	const names = $derived(sample.map((r) => r.name));
	const preview = $derived(
		names.length === 0
			? ''
			: names.length <= 3
				? names.join(', ')
				: `${names.slice(0, 3).join(', ')} +${count - 3} more`
	);

	async function confirmDelete() {
		if (count === 0) return;

		deleting = true;
		const results = await Promise.all(roleIds.map((id) => deleteTenantRole(tenantId, id)));
		deleting = false;

		const failed = results.filter(([err]) => err).length;
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });

		if (failed > 0) {
			toast.error(`Couldn't delete ${failed} of ${count} role${count === 1 ? '' : 's'}.`);
		} else {
			toast.success(`Deleted ${count} role${count === 1 ? '' : 's'}`);
		}

		onDeleted();
		onOpenChange(false);
	}
</script>

<Dialog.Root {open} onOpenChange={(value) => !value && onOpenChange(false)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="bg-destructive/10 text-destructive mb-1 flex size-11 items-center justify-center rounded-full"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>
				Delete {count} role{count === 1 ? '' : 's'}?
			</Dialog.Title>
			<Dialog.Description>
				This removes {count === 1 ? 'this role' : 'these roles'} and all their assignments.
				This action can't be undone.
			</Dialog.Description>
		</Dialog.Header>

		{#if preview}
			<div class="bg-muted/40 text-muted-foreground rounded-lg border p-3 text-sm">
				{preview}
			</div>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => onOpenChange(false)}>
				Cancel
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				Delete {count === 1 ? 'role' : `${count} roles`}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
