<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { m } from '$lib/paraglide/messages.js';
	import { deleteTenantGroup, type TenantGroupView } from '$lib/services/admin/groups';

	type Props = {
		tenantId: string;
		groupIds: string[];
		sample: TenantGroupView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onDeleted: () => void;
	};

	let { tenantId, groupIds, sample, open, onOpenChange, onDeleted }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	const count = $derived(groupIds.length);
	const names = $derived(sample.map((g) => g.name));
	const groupNoun = $derived(count === 1 ? m.groups_noun_singular() : m.groups_noun_plural());
	const preview = $derived(
		names.length === 0
			? ''
			: names.length <= 3
				? names.join(', ')
				: `${names.slice(0, 3).join(', ')} ${m.groups_bulk_delete_more_suffix({ count: count - 3 })}`
	);

	async function confirmDelete() {
		if (count === 0) return;

		deleting = true;
		const results = await Promise.all(groupIds.map((id) => deleteTenantGroup(tenantId, id)));
		deleting = false;

		const failed = results.filter(([err]) => err).length;
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });

		if (failed > 0) {
			toast.error(m.groups_bulk_delete_error_toast({ failed, count, noun: groupNoun }));
		} else {
			toast.success(m.groups_bulk_delete_success_toast({ count, noun: groupNoun }));
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
				{m.groups_bulk_delete_dialog_title({ count, noun: groupNoun })}
			</Dialog.Title>
			<Dialog.Description>
				{m.groups_bulk_delete_dialog_description({
					target: count === 1 ? m.groups_bulk_delete_target_singular() : m.groups_bulk_delete_target_plural()
				})}
			</Dialog.Description>
		</Dialog.Header>

		{#if preview}
			<div class="bg-muted/40 text-muted-foreground rounded-lg border p-3 text-sm">
				{preview}
			</div>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => onOpenChange(false)}>
				{m.groups_cancel()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.groups_bulk_delete_confirm_button({
					target: count === 1 ? m.groups_noun_singular() : `${count} ${m.groups_noun_plural()}`
				})}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
