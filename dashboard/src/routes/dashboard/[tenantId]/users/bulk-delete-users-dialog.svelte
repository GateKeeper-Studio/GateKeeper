<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { deleteTenantUser, type TenantUserView } from '$lib/services/admin/users';
	import { userFullName } from './display.js';

	type Props = {
		tenantId: string;
		// All selected user ids (may span pages); `sample` is the subset whose full
		// records are loaded on the current page, used only for naming a few.
		userIds: string[];
		sample: TenantUserView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onDeleted: () => void;
	};

	let { tenantId, userIds, sample, open, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	const count = $derived(userIds.length);
	const names = $derived(sample.map(userFullName));
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
		// The IdP has no bulk endpoint, so fan out single deletes and tally failures.
		const results = await Promise.all(userIds.map((id) => deleteTenantUser(tenantId, id)));
		deleting = false;

		const failed = results.filter(([err]) => err).length;
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId] });

		if (failed > 0) {
			toast.error(`Couldn't delete ${failed} of ${count} user${count === 1 ? '' : 's'}.`);
		} else {
			toast.success(`Deleted ${count} user${count === 1 ? '' : 's'}`);
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
				Delete {count} user{count === 1 ? '' : 's'}?
			</Dialog.Title>
			<Dialog.Description>
				This removes {count === 1 ? 'this user' : 'these users'} from the tenant and revokes their access.
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
				Delete {count === 1 ? 'user' : `${count} users`}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
