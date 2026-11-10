<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import { page } from '$app/state';

	import UserPlusIcon from '@lucide/svelte/icons/user-plus';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import {
		deactivateAdmin,
		inviteAdmin,
		listInvitations,
		listOrganizationAdmins,
		reactivateAdmin,
		resetAdminMfa,
		revokeInvitation,
		type CreatedInvitation,
		type OrganizationAdmin
	} from '$lib/services/admin/organization-admins';
	import type { ApiError } from '$lib/services/types';

	const queryClient = useQueryClient();
	const myId = $derived((page.data as { admin?: { id: string } }).admin?.id);

	const adminsQuery = createQuery(() => ({
		queryKey: ['organization', 'admins'],
		queryFn: async () => {
			const [data, err] = await listOrganizationAdmins();
			if (err) throw err;
			return data;
		}
	}));
	const invitationsQuery = createQuery(() => ({
		queryKey: ['organization', 'invitations'],
		queryFn: async () => {
			const [data, err] = await listInvitations();
			if (err) throw err;
			return data;
		}
	}));

	function refresh() {
		queryClient.invalidateQueries({ queryKey: ['organization', 'admins'] });
		queryClient.invalidateQueries({ queryKey: ['organization', 'invitations'] });
	}

	function messageOf(err: ApiError) {
		return err.data?.message || err.message || m.settings_mfa_error();
	}

	const formatDate = (iso: string) => new Date(iso).toLocaleDateString();

	// ── One dialog for every action that needs the current password ──────────────
	type Action = 'invite' | 'deactivate' | 'reactivate' | 'reset-mfa';
	let open = $state(false);
	let action = $state<Action>('invite');
	let target = $state<OrganizationAdmin | null>(null);
	let email = $state('');
	let password = $state('');
	let busy = $state(false);
	let error = $state('');
	let created = $state<CreatedInvitation | null>(null);
	let copied = $state(false);

	function begin(next: Action, admin: OrganizationAdmin | null = null) {
		action = next;
		target = admin;
		email = '';
		password = '';
		error = '';
		created = null;
		copied = false;
		open = true;
	}

	function onOpenChangeComplete(isOpen: boolean) {
		if (!isOpen) {
			password = '';
			created = null;
		}
	}

	const titles: Record<Action, () => string> = {
		invite: () => m.org_admins_invite_title(),
		deactivate: () => m.org_admins_deactivate_title(),
		reactivate: () => m.org_admins_reactivate_title(),
		'reset-mfa': () => m.org_admins_reset_mfa_title()
	};
	const descriptions: Record<Action, () => string> = {
		invite: () => m.org_admins_invite_description(),
		deactivate: () => m.org_admins_deactivate_description({ email: target?.email ?? '' }),
		reactivate: () => m.org_admins_reactivate_description({ email: target?.email ?? '' }),
		'reset-mfa': () => m.org_admins_reset_mfa_description({ email: target?.email ?? '' })
	};

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (action === 'invite' && email.trim().length === 0) {
			error = m.settings_account_email_required();
			return;
		}
		if (password.length === 0) {
			error = m.settings_mfa_password_required();
			return;
		}
		error = '';
		busy = true;

		if (action === 'invite') {
			const [data, err] = await inviteAdmin(email.trim(), password);
			busy = false;
			password = '';
			if (err || !data) {
				error = err ? messageOf(err) : m.settings_mfa_error();
				return;
			}
			created = data;
			refresh();
			return;
		}

		const id = target!.id;
		const call =
			action === 'deactivate'
				? deactivateAdmin(id, password)
				: action === 'reactivate'
					? reactivateAdmin(id, password)
					: resetAdminMfa(id, password);
		const [err] = await call;
		busy = false;
		password = '';
		if (err) {
			error = messageOf(err);
			return;
		}
		toast.success(m.org_admins_done());
		open = false;
		refresh();
	}

	async function copyLink() {
		if (!created) return;
		await navigator.clipboard.writeText(created.url);
		copied = true;
	}

	async function revoke(id: string) {
		const [err] = await revokeInvitation(id);
		if (err) {
			toast.error(messageOf(err));
			return;
		}
		refresh();
	}
</script>

<Card.Root class="mt-6 max-w-[640px]">
	<Card.Header class="flex flex-row items-start justify-between gap-4">
		<div class="space-y-1.5">
			<Card.Title>{m.org_admins_title()}</Card.Title>
			<Card.Description>{m.org_admins_description()}</Card.Description>
		</div>
		<Button size="sm" onclick={() => begin('invite')}>
			<UserPlusIcon />
			{m.org_admins_invite_button()}
		</Button>
	</Card.Header>
	<Card.Content class="space-y-6">
		{#if adminsQuery.isPending}
			<div class="flex items-center gap-2 text-sm text-muted-foreground"><LoadingSpinner /></div>
		{:else if adminsQuery.isError}
			<ErrorAlert title={m.org_admins_load_error()} message={adminsQuery.error.message} />
		{:else if adminsQuery.data}
			<ul class="divide-y rounded-lg border">
				{#each adminsQuery.data as admin (admin.id)}
					<li class="flex flex-wrap items-center justify-between gap-3 p-3">
						<div class="min-w-0">
							<div class="flex flex-wrap items-center gap-2">
								<span class="truncate font-medium">{admin.name || admin.email}</span>
								{#if admin.id === myId}<Badge variant="outline">{m.org_admins_you()}</Badge>{/if}
								{#if !admin.isActive}
									<Badge variant="secondary">{m.settings_account_status_inactive()}</Badge>
								{/if}
								{#if admin.lockedUntil}
									<Badge variant="destructive">{m.org_admins_locked()}</Badge>
								{/if}
								{#if admin.mfaEnabled}
									<Badge variant="default">{m.org_admins_mfa_on()}</Badge>
								{:else}
									<Badge variant="outline">{m.org_admins_mfa_off()}</Badge>
								{/if}
							</div>
							<p class="truncate text-xs text-muted-foreground">
								{admin.email} ·
								{admin.lastSignInAt
									? m.org_admins_last_sign_in({ date: formatDate(admin.lastSignInAt) })
									: m.org_admins_never_signed_in()}
							</p>
						</div>
						{#if admin.id !== myId}
							<div class="flex gap-2">
								{#if admin.mfaEnabled}
									<Button size="sm" variant="outline" onclick={() => begin('reset-mfa', admin)}>
										{m.org_admins_reset_mfa_button()}
									</Button>
								{/if}
								{#if admin.isActive}
									<Button size="sm" variant="outline" onclick={() => begin('deactivate', admin)}>
										{m.org_admins_deactivate_button()}
									</Button>
								{:else}
									<Button size="sm" variant="outline" onclick={() => begin('reactivate', admin)}>
										{m.org_admins_reactivate_button()}
									</Button>
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		{#if invitationsQuery.data && invitationsQuery.data.length > 0}
			<div class="space-y-2">
				<h3 class="text-sm font-medium">{m.org_admins_pending_title()}</h3>
				<ul class="divide-y rounded-lg border">
					{#each invitationsQuery.data as invitation (invitation.id)}
						<li class="flex items-center justify-between gap-3 p-3 text-sm">
							<div class="min-w-0">
								<p class="truncate">{invitation.email}</p>
								<p class="text-xs text-muted-foreground">
									{m.org_admins_expires({ date: formatDate(invitation.expiresAt) })}
								</p>
							</div>
							<Button size="sm" variant="ghost" onclick={() => revoke(invitation.id)}>
								{m.org_admins_revoke_button()}
							</Button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</Card.Content>
</Card.Root>

<Dialog.Root bind:open {onOpenChangeComplete}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{titles[action]()}</Dialog.Title>
			<Dialog.Description>
				{created ? m.org_admins_link_description() : descriptions[action]()}
			</Dialog.Description>
		</Dialog.Header>

		{#if created}
			<div class="space-y-3">
				<Input readonly value={created.url} class="font-mono text-xs" />
				<p class="text-xs text-muted-foreground">
					{m.org_admins_expires({ date: formatDate(created.expiresAt) })}
				</p>
			</div>
			<Dialog.Footer>
				<Button variant="outline" onclick={copyLink}>
					{copied ? m.settings_mfa_copied() : m.settings_mfa_copy_codes()}
				</Button>
				<Button onclick={() => (open = false)}>{m.settings_mfa_codes_done()}</Button>
			</Dialog.Footer>
		{:else}
			<form onsubmit={submit} class="grid gap-4">
				{#if action === 'invite'}
					<div class="space-y-2">
						<Label for="invite-admin-email">{m.settings_account_email_label()}</Label>
						<Input id="invite-admin-email" type="email" autocomplete="off" bind:value={email} />
					</div>
				{/if}
				<div class="space-y-2">
					<Label for="org-admin-password">{m.settings_mfa_password_label()}</Label>
					<Input
						id="org-admin-password"
						type="password"
						autocomplete="current-password"
						bind:value={password}
					/>
				</div>
				{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
				<Dialog.Footer>
					<Button type="button" variant="outline" disabled={busy} onclick={() => (open = false)}>
						{m.settings_mfa_cancel()}
					</Button>
					<Button
						type="submit"
						variant={action === 'deactivate' || action === 'reset-mfa' ? 'destructive' : 'default'}
						disabled={busy}
						class="relative"
					>
						{#if busy}<LoadingSpinner class="absolute left-3" />{/if}
						{m.settings_mfa_continue()}
					</Button>
				</Dialog.Footer>
			</form>
		{/if}
	</Dialog.Content>
</Dialog.Root>
