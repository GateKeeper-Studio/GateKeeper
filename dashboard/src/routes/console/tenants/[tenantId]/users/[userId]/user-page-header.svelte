<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';

	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';

	import { Button } from '$lib/components/ui/button/index.js';
	import CopyIdButton from '$lib/components/copy-id-button.svelte';
	import type { TenantUserView } from '$lib/services/admin/users';

	import UserAvatar from '../user-avatar.svelte';
	import DeleteUserDialog from '../delete-user-dialog.svelte';
	import { userFullName } from '../display.js';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		user: TenantUserView;
		tenantId: string;
		userId: string;
		listHref: string;
	};

	let { user, tenantId, userId, listHref }: Props = $props();

	let deleteTarget = $state<TenantUserView | null>(null);
	let emailCopied = $state(false);

	async function copyEmail() {
		try {
			await navigator.clipboard.writeText(user.email);
			emailCopied = true;
			setTimeout(() => (emailCopied = false), 1500);
			toast.success(m.users_email_copied_success());
		} catch {
			toast.error(m.users_copy_clipboard_error());
		}
	}
</script>

<div class="flex flex-wrap items-start justify-between gap-4">
	<div class="flex items-center gap-4">
		<UserAvatar {user} class="size-14 text-lg" />
		<div class="min-w-0">
			<h1 class="truncate text-3xl font-bold tracking-tight">{userFullName(user)}</h1>
			<button
				type="button"
				onclick={copyEmail}
				class="text-muted-foreground hover:text-foreground group mt-1 flex items-center gap-1.5 text-sm"
			>
				<span class="truncate">{user.email}</span>
				{#if emailCopied}
					<CheckIcon class="size-3.5 text-emerald-600 dark:text-emerald-400" />
				{:else}
					<CopyIcon class="size-3.5 opacity-0 transition group-hover:opacity-100" />
				{/if}
			</button>
		</div>
	</div>

	<div class="flex gap-2">
		<CopyIdButton id={userId} />
		<Button variant="outline" href={`${listHref}/${userId}/edit-user`}>
			<PencilIcon /> {m.users_action_edit()}
		</Button>
		<Button variant="destructive" onclick={() => (deleteTarget = user)}>
			<Trash2Icon /> {m.users_action_delete()}
		</Button>
	</div>
</div>

<DeleteUserDialog
	{tenantId}
	user={deleteTarget}
	onOpenChange={() => (deleteTarget = null)}
	onDeleted={() => goto(listHref)}
/>
