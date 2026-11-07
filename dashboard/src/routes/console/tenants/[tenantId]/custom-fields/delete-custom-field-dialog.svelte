<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import {
		deleteCustomField,
		type CustomFieldDefinitionView
	} from '$lib/services/admin/custom-fields';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		// The field pending deletion; null keeps the dialog closed.
		field: CustomFieldDefinitionView | null;
		onOpenChange: (open: boolean) => void;
	};

	let { tenantId, field, onOpenChange }: Props = $props();

	const queryClient = useQueryClient();
	let deleting = $state(false);

	async function confirmDelete() {
		if (!field) return;

		deleting = true;
		const [err] = await deleteCustomField(tenantId, field.id);
		deleting = false;

		if (err) {
			toast.error(err.data?.message || m.custom_fields_delete_error());
			return;
		}

		toast.success(m.custom_fields_delete_success());
		queryClient.invalidateQueries({ queryKey: ['custom-fields', tenantId] });
		onOpenChange(false);
	}
</script>

<Dialog.Root open={!!field} onOpenChange={(open) => !open && onOpenChange(false)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="mb-1 flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>{m.custom_fields_delete_confirm_title()}</Dialog.Title>
			<Dialog.Description>
				{m.custom_fields_delete_confirm_description()}
			</Dialog.Description>
		</Dialog.Header>

		{#if field}
			<div class="flex items-center gap-3 rounded-lg border bg-muted/40 p-3">
				<div class="min-w-0">
					<p class="truncate text-sm font-medium">{field.label}</p>
					<p class="truncate font-mono text-xs text-muted-foreground">{field.fieldKey}</p>
				</div>
			</div>
		{/if}

		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => onOpenChange(false)}>
				{m.oauth_clients_action_cancel()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.custom_fields_action_delete()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
