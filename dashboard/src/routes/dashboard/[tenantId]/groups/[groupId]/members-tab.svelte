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

	import { listGroupMembers, removeGroupMember } from '$lib/services/admin/groups';
	import type { TenantUserView } from '$lib/services/admin/users';
	import { userFullName } from '../../users/display.js';
	import UserAvatar from '../../users/user-avatar.svelte';
	import AddMembersDialog from './add-members-dialog.svelte';

	type Props = { tenantId: string; groupId: string };
	let { tenantId, groupId }: Props = $props();

	const queryClient = useQueryClient();

	let pageIndex = $state(0);
	const pageSize = 10;
	let addMembersOpen = $state(false);
	let removingId = $state<string | null>(null);

	const membersQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId, groupId, 'members', pageIndex, pageSize],
		enabled: !!tenantId && !!groupId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const [data, err] = await listGroupMembers(tenantId, groupId, {
				page: pageIndex + 1,
				pageSize
			});
			if (err) throw err;
			return data;
		}
	}));

	const members = $derived(membersQuery.data?.items ?? []);
	const total = $derived(membersQuery.data?.total ?? 0);
	const pageCount = $derived(Math.max(1, Math.ceil(total / pageSize)));
	const rangeStart = $derived(total === 0 ? 0 : pageIndex * pageSize + 1);
	const rangeEnd = $derived(Math.min(total, (pageIndex + 1) * pageSize));

	async function removeMember(user: TenantUserView) {
		removingId = user.user_id;
		const [err] = await removeGroupMember(tenantId, groupId, user.user_id);
		removingId = null;

		if (err) {
			toast.error(err.data?.message || 'Could not remove member.');
			return;
		}

		toast.success(`${userFullName(user)} removed from group`);
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId, groupId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId, groupId, 'members'] });
	}
</script>

<div class="mt-4 space-y-4">
	<div class="flex items-center justify-between">
		<p class="text-muted-foreground text-sm">
			{#if total}{total} member{total === 1 ? '' : 's'}{/if}
		</p>
		<Button size="sm" onclick={() => (addMembersOpen = true)}>
			<PlusIcon />
			Add members
		</Button>
	</div>

	{#if membersQuery.isError}
		<ErrorAlert title="Couldn't load members" message={membersQuery.error.message} />
	{:else}
		<div
			class={[
				'overflow-hidden rounded-xl border',
				membersQuery.isPlaceholderData && 'opacity-60 transition-opacity'
			]}
		>
			<Table.Root>
				<Table.Header>
					<Table.Row class="bg-muted/40 hover:bg-muted/40">
						<Table.Head>Member</Table.Head>
						<Table.Head class="hidden sm:table-cell">Email</Table.Head>
						<Table.Head class="w-12"></Table.Head>
					</Table.Row>
				</Table.Header>

				<Table.Body>
					{#if membersQuery.isPending}
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
					{:else if members.length}
						{#each members as member (member.user_id)}
							<Table.Row>
								<Table.Cell>
									<div class="flex items-center gap-3">
										<UserAvatar user={member} />
										<a
											href={`/dashboard/${tenantId}/users/${member.user_id}`}
											class="hover:text-primary truncate font-medium hover:underline"
										>
											{userFullName(member)}
										</a>
									</div>
								</Table.Cell>
								<Table.Cell class="text-muted-foreground hidden truncate text-sm sm:table-cell">
									{member.email}
								</Table.Cell>
								<Table.Cell>
									<div class="flex justify-end">
										<Button
											variant="ghost"
											size="icon"
											class="text-muted-foreground hover:text-destructive size-8"
											disabled={removingId === member.user_id}
											onclick={() => removeMember(member)}
										>
											{#if removingId === member.user_id}
												<LoadingSpinner />
											{:else}
												<Trash2Icon class="size-4" />
											{/if}
											<span class="sr-only">Remove member</span>
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
									<p class="text-sm">No members yet.</p>
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

<AddMembersDialog
	{tenantId}
	{groupId}
	currentMembers={members}
	open={addMembersOpen}
	onOpenChange={(v) => (addMembersOpen = v)}
/>
