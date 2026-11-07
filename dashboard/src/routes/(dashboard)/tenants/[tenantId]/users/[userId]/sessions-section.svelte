<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import {
		listUserSessions,
		revokeAllUserSessions,
		revokeUserSession
	} from '$lib/services/admin/sessions';
	import { m } from '$lib/paraglide/messages.js';

	let { tenantId, userId }: { tenantId: string; userId: string } = $props();

	const queryClient = useQueryClient();
	const queryKey = $derived(['tenant-users', tenantId, userId, 'sessions']);

	// The apps this user is signed in to: one row per active refresh-token family.
	const sessionsQuery = createQuery(() => ({
		queryKey,
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await listUserSessions(tenantId, userId);
			if (err) throw err;
			return data;
		}
	}));

	let revokingId = $state<string | null>(null);
	let confirmAllOpen = $state(false);
	let revokingAll = $state(false);

	async function revokeOne(sessionId: string) {
		revokingId = sessionId;
		const [err] = await revokeUserSession(tenantId, userId, sessionId);
		revokingId = null;

		if (err) {
			toast.error(err.message || m.users_sessions_revoke_error());
			return;
		}
		toast.success(m.users_sessions_revoke_success());
		queryClient.invalidateQueries({ queryKey });
	}

	async function revokeAll() {
		revokingAll = true;
		const [err] = await revokeAllUserSessions(tenantId, userId);
		revokingAll = false;
		confirmAllOpen = false;

		if (err) {
			toast.error(err.message || m.users_sessions_revoke_error());
			return;
		}
		toast.success(m.users_sessions_revoke_all_success());
		queryClient.invalidateQueries({ queryKey });
	}

	function when(iso: string): string {
		return new Date(iso).toLocaleString();
	}
</script>

<div class="mt-4 flex flex-wrap items-end justify-between gap-3">
	<div class="space-y-1">
		<h2 class="text-lg font-semibold">{m.users_sessions_title()}</h2>
		<p class="max-w-2xl text-sm text-muted-foreground">{m.users_sessions_description()}</p>
	</div>
	{#if sessionsQuery.data && sessionsQuery.data.length > 0}
		<Button variant="outline" onclick={() => (confirmAllOpen = true)}>
			<LogOutIcon class="size-4" />
			{m.users_sessions_revoke_all_button()}
		</Button>
	{/if}
</div>

<div class="mt-4">
	{#if sessionsQuery.isPending}
		<Skeleton class="h-32 w-full rounded-xl" />
	{:else if sessionsQuery.isError}
		<ErrorAlert title={m.users_sessions_load_error_title()} message={sessionsQuery.error.message} />
	{:else if sessionsQuery.data.length === 0}
		<p class="rounded-xl border p-4 text-sm text-muted-foreground">{m.users_sessions_empty()}</p>
	{:else}
		<div class="overflow-x-auto rounded-xl border">
			<table class="w-full text-left text-sm">
				<thead class="bg-muted/50 text-muted-foreground">
					<tr>
						<th class="px-4 py-2 font-medium">{m.users_sessions_col_app()}</th>
						<th class="px-4 py-2 font-medium">{m.users_sessions_col_signed_in()}</th>
						<th class="px-4 py-2 font-medium">{m.users_sessions_col_last_active()}</th>
						<th class="px-4 py-2 font-medium">{m.users_sessions_col_expires()}</th>
						<th class="px-4 py-2"><span class="sr-only">{m.users_sessions_col_actions()}</span></th>
					</tr>
				</thead>
				<tbody>
					{#each sessionsQuery.data as session (session.id)}
						<tr class="border-t">
							<td class="px-4 py-2">
								<div class="font-medium">{session.client_name ?? session.client_id}</div>
								<div class="text-xs text-muted-foreground">{session.scopes.join(' ')}</div>
							</td>
							<td class="px-4 py-2 whitespace-nowrap">{when(session.signed_in_at)}</td>
							<td class="px-4 py-2 whitespace-nowrap">{when(session.last_active_at)}</td>
							<td class="px-4 py-2 whitespace-nowrap">{when(session.expires_at)}</td>
							<td class="px-4 py-2 text-right">
								<Button
									variant="outline"
									size="sm"
									disabled={revokingId !== null || revokingAll}
									onclick={() => revokeOne(session.id)}
								>
									{#if revokingId === session.id}<LoadingSpinner />{/if}
									{m.users_sessions_revoke_button()}
								</Button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
	<p class="mt-2 text-xs text-muted-foreground">{m.users_sessions_access_token_note()}</p>
</div>

<Dialog.Root bind:open={confirmAllOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div
				class="mb-1 flex size-11 items-center justify-center rounded-full bg-destructive/10 text-destructive"
			>
				<TriangleAlertIcon class="size-5" />
			</div>
			<Dialog.Title>{m.users_sessions_revoke_all_title()}</Dialog.Title>
			<Dialog.Description>{m.users_sessions_revoke_all_description()}</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={revokingAll} onclick={() => (confirmAllOpen = false)}>
				{m.users_action_cancel()}
			</Button>
			<Button variant="destructive" disabled={revokingAll} onclick={revokeAll}>
				{#if revokingAll}<LoadingSpinner />{/if}
				{m.users_sessions_revoke_all_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
