<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import KeyRoundIcon from '@lucide/svelte/icons/key-round';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { resetUserPassword } from '$lib/services/admin/users';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		userId: string;
		// Where the temporary password goes; shown so the admin can check it first.
		email: string;
		open: boolean;
		onOpenChange: (open: boolean) => void;
	};

	let { tenantId, userId, email, open, onOpenChange }: Props = $props();

	const queryClient = useQueryClient();
	let sending = $state(false);

	// The IdP makes the temporary password and emails it; this page never sees it.
	async function confirmReset() {
		sending = true;
		const [data, err] = await resetUserPassword(tenantId, userId);
		sending = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success(m.users_reset_password_success({ email: data?.email ?? email }));
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId, userId] });
		onOpenChange(false);
	}
</script>

<Dialog.Root {open} onOpenChange={(next) => !sending && onOpenChange(next)}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="mb-1 flex size-11 items-center justify-center rounded-full bg-amber-500/10 text-amber-600"
			>
				<KeyRoundIcon class="size-5" />
			</div>
			<Dialog.Title>{m.users_reset_password_title()}</Dialog.Title>
			<Dialog.Description>
				{m.users_reset_password_description({ email })}
			</Dialog.Description>
		</Dialog.Header>

		<ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
			<li>{m.users_reset_password_consequences()}</li>
			<li>{m.users_reset_password_private()}</li>
		</ul>

		<Dialog.Footer>
			<Button variant="outline" disabled={sending} onclick={() => onOpenChange(false)}>
				{m.users_action_cancel()}
			</Button>
			<Button disabled={sending} onclick={confirmReset}>
				{#if sending}<LoadingSpinner />{/if}
				{m.users_reset_password_confirm()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
