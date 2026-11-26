<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { createQuery } from '@tanstack/svelte-query';

	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';

	import { Button } from '$lib/components/ui/button/index.js';
	import CopyIdButton from '$lib/components/copy-id-button.svelte';
	import { getUserAuthenticationMethods, type TenantUserView } from '$lib/services/admin/users';

	import UserAvatar from '../user-avatar.svelte';
	import DeleteUserDialog from '../delete-user-dialog.svelte';
	import ResetPasswordDialog from './reset-password-dialog.svelte';
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
	let resetOpen = $state(false);

	// Same query (and cache entry) as the Security tab's sign-in methods: the reset is
	// offered only to an account that has a password.
	const methodsQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, userId, 'authentication-methods'],
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await getUserAuthenticationMethods(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));
	const hasPassword = $derived(!!methodsQuery.data?.password.set);

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
				class="group mt-1 flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
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

	<div class="flex gap-2" data-tour="user-actions">
		<CopyIdButton id={userId} />
		<Button variant="outline" href={`${listHref}/${userId}/edit-user`}>
			<PencilIcon />
			{m.users_action_edit()}
		</Button>
		{#if hasPassword}
			<Button variant="outline" onclick={() => (resetOpen = true)}>
				<KeyRoundIcon />
				{m.users_reset_password_button()}
			</Button>
		{/if}
		<Button variant="destructive" onclick={() => (deleteTarget = user)}>
			<Trash2Icon />
			{m.users_action_delete()}
		</Button>
	</div>
</div>

<DeleteUserDialog
	{tenantId}
	user={deleteTarget}
	onOpenChange={() => (deleteTarget = null)}
	onDeleted={() => goto(listHref)}
/>

<ResetPasswordDialog
	{tenantId}
	{userId}
	email={user.email}
	open={resetOpen}
	onOpenChange={(open) => (resetOpen = open)}
/>
