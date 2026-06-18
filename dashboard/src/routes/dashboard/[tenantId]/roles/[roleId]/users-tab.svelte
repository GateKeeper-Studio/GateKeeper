<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { keepPreviousData } from '@tanstack/svelte-query';

	import PlusIcon from '@lucide/svelte/icons/plus';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import UsersIcon from '@lucide/svelte/icons/users';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

	import { toast } from 'svelte-sonner';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { listRoleUsers, unassignRoleUser } from '$lib/services/admin/roles';
	import type { TenantUserView } from '$lib/services/admin/users';
	import { userFullName } from '../../users/display.js';
	import UserAvatar from '../../users/user-avatar.svelte';
	import AssignUsersDialog from './assign-users-dialog.svelte';

	type Props = { tenantId: string; roleId: string };
	let { tenantId, roleId }: Props = $props();

	const queryClient = useQueryClient();

	let pageIndex = $state(0);
	const pageSize = 10;
	let assignUsersOpen = $state(false);
	let removingId = $state<string | null>(null);

	const usersQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId, roleId, 'users', pageIndex, pageSize],
		enabled: !!tenantId && !!roleId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listRoleUsers(tenantId, roleId, {
				page: pageIndex + 1,
				pageSize
			});
			if (err) throw err;
			return data;
		}
	}));

	const users = $derived(usersQuery.data?.items ?? []);
	const total = $derived(usersQuery.data?.total ?? 0);
	const pageCount = $derived(Math.max(1, Math.ceil(total / pageSize)));
	const rangeStart = $derived(total === 0 ? 0 : pageIndex * pageSize + 1);
	const rangeEnd = $derived(Math.min(total, (pageIndex + 1) * pageSize));

	async function unassignUser(user: TenantUserView) {
		removingId = user.user_id;
		const [err] = await unassignRoleUser(tenantId, roleId, user.user_id);
		removingId = null;

		if (err) {
			toast.error(err.data?.message || 'Could not unassign user.');
			return;
		}

		toast.success(`${userFullName(user)} unassigned from role`);
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId, roleId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId, roleId, 'users'] });
	}
</script>

<div class="mt-4 space-y-4">
	<div class="flex items-center justify-between">
		<p class="text-muted-foreground text-sm">
			{#if total}{total} user{total === 1 ? '' : 's'}{/if}
		</p>
		<Button size="sm" onclick={() => (assignUsersOpen = true)}>
			<PlusIcon />
			Assign users
		</Button>
	</div>

	{#if usersQuery.isError}
		<ErrorAlert title="Couldn't load users" message={usersQuery.error.message} />
	{:else}
		<div
			class={[
				'overflow-hidden rounded-xl border',
				usersQuery.isPlaceholderData && 'opacity-60 transition-opacity'
			]}
		>
			<Table.Root>
				<Table.Header>
					<Table.Row class="bg-muted/40 hover:bg-muted/40">
						<Table.Head>User</Table.Head>
						<Table.Head class="hidden sm:table-cell">Email</Table.Head>
						<Table.Head class="w-12"></Table.Head>
					</Table.Row>
				</Table.Header>

				<Table.Body>
					{#if usersQuery.isPending}
						{#each Array(5) as _, i (i)}
							<Table.Row class="hover:bg-transparent">
								<Table.Cell>
									<div class="flex items-center gap-3">
										<Skeleton class="size-9 rounded-full" />
										<Skeleton class="h-3.5 w-32" />
									</div>
								</Table.Cell>
								<Table.Cell class="hidden sm:table-cell"><Skeleton class="h-3.5 w-40" /></Table.Cell>
								<Table.Cell><Skeleton class="ml-auto size-8 rounded" /></Table.Cell>
							</Table.Row>
						{/each}
					{:else if users.length}
						{#each users as user (user.user_id)}
							<Table.Row>
								<Table.Cell>
									<div class="flex items-center gap-3">
										<UserAvatar {user} />
										<a
											href={`/dashboard/${tenantId}/users/${user.user_id}`}
											class="hover:text-primary truncate font-medium hover:underline"
										>
											{userFullName(user)}
										</a>
									</div>
								</Table.Cell>
								<Table.Cell class="text-muted-foreground hidden truncate text-sm sm:table-cell">
									{user.email}
								</Table.Cell>
								<Table.Cell>
									<div class="flex justify-end">
										<Button
											variant="ghost"
											size="icon"
											class="text-muted-foreground hover:text-destructive size-8"
											disabled={removingId === user.user_id}
											onclick={() => unassignUser(user)}
										>
											{#if removingId === user.user_id}
												<LoadingSpinner />
											{:else}
												<Trash2Icon class="size-4" />
											{/if}
											<span class="sr-only">Unassign user</span>
										</Button>
									</div>
								</Table.Cell>
							</Table.Row>
						{/each}
					{:else}
						<Table.Row class="hover:bg-transparent">
							<Table.Cell colspan={3} class="h-32 text-center">
								<div class="text-muted-foreground flex flex-col items-center gap-2">
									<UsersIcon class="size-7 opacity-60" />
									<p class="text-sm">No users assigned yet.</p>
								</div>
							</Table.Cell>
						</Table.Row>
					{/if}
				</Table.Body>
			</Table.Root>
		</div>

		{#if total > pageSize}
			<div class="flex items-center justify-between">
				<p class="text-muted-foreground text-sm">
					Showing <span class="text-foreground font-medium">{rangeStart}–{rangeEnd}</span> of
					<span class="text-foreground font-medium">{total}</span>
				</p>
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-sm">Page {pageIndex + 1} of {pageCount}</span>
					<Button
						variant="outline"
						size="icon"
						class="size-8"
						disabled={pageIndex === 0}
						onclick={() => (pageIndex -= 1)}
					>
						<ChevronLeftIcon />
					</Button>
					<Button
						variant="outline"
						size="icon"
						class="size-8"
						disabled={pageIndex >= pageCount - 1}
						onclick={() => (pageIndex += 1)}
					>
						<ChevronRightIcon />
					</Button>
				</div>
			</div>
		{/if}
	{/if}
</div>

<AssignUsersDialog
	{tenantId}
	{roleId}
	currentUsers={users}
	open={assignUsersOpen}
	onOpenChange={(v) => (assignUsersOpen = v)}
/>
