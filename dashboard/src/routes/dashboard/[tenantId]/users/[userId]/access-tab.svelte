<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import ShieldIcon from '@lucide/svelte/icons/shield';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import {
		listUserGroups,
		removeGroupMember,
		type TenantGroupView
	} from '$lib/services/admin/groups';
	import { listUserRoles, unassignRoleUser, type TenantRoleView } from '$lib/services/admin/roles';
	import GroupAvatar from '../../groups/group-avatar.svelte';
	import RoleAvatar from '../../roles/role-avatar.svelte';
	import AssignRoleDialog from './assign-role-dialog.svelte';
	import AssignGroupDialog from './assign-group-dialog.svelte';

	type Props = { tenantId: string; userId: string };
	let { tenantId, userId }: Props = $props();

	const queryClient = useQueryClient();

	const rolesQueryKey = $derived(['tenant-users', tenantId, userId, 'roles'] as const);
	const groupsQueryKey = $derived(['tenant-users', tenantId, userId, 'groups'] as const);

	const rolesQuery = createQuery(() => ({
		queryKey: rolesQueryKey,
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await listUserRoles(tenantId, userId);
			if (err) throw err;
			return data ?? [];
		}
	}));

	const groupsQuery = createQuery(() => ({
		queryKey: groupsQueryKey,
		enabled: !!tenantId && !!userId,
		queryFn: async () => {
			const [data, err] = await listUserGroups(tenantId, userId);
			if (err) throw err;
			return data ?? [];
		}
	}));

	const roles = $derived(rolesQuery.data ?? []);
	const groups = $derived(groupsQuery.data ?? []);
	const rolesLoading = $derived(
		rolesQuery.data === undefined && rolesQuery.fetchStatus === 'fetching'
	);
	const groupsLoading = $derived(
		groupsQuery.data === undefined && groupsQuery.fetchStatus === 'fetching'
	);

	let assignRoleOpen = $state(false);
	let assignGroupOpen = $state(false);
	let removingRoleId = $state<string | null>(null);
	let removingGroupId = $state<string | null>(null);

	async function unassignRole(role: TenantRoleView) {
		removingRoleId = role.role_id;
		const [err] = await unassignRoleUser(tenantId, role.role_id, userId);
		removingRoleId = null;

		if (err) {
			toast.error(err.data?.message || 'Could not unassign role.');
			return;
		}

		toast.success(`Role "${role.name}" unassigned`);
		queryClient.setQueryData<TenantRoleView[]>(rolesQueryKey, (current) =>
			(current ?? []).filter((item) => item.role_id !== role.role_id)
		);
		queryClient.invalidateQueries({ queryKey: rolesQueryKey });
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });
	}

	async function removeGroup(group: TenantGroupView) {
		removingGroupId = group.group_id;
		const [err] = await removeGroupMember(tenantId, group.group_id, userId);
		removingGroupId = null;

		if (err) {
			toast.error(err.data?.message || 'Could not remove from group.');
			return;
		}

		toast.success(`Removed from "${group.name}"`);
		queryClient.setQueryData<TenantGroupView[]>(groupsQueryKey, (current) =>
			(current ?? []).filter((item) => item.group_id !== group.group_id)
		);
		queryClient.invalidateQueries({ queryKey: groupsQueryKey });
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });
	}
</script>

<div class="grid items-start gap-4 lg:grid-cols-2">
	<!-- Roles — live data -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-start justify-between gap-3">
				<div class="space-y-1.5">
					<Card.Title class="flex items-center gap-2">
						<ShieldIcon class="size-4 text-muted-foreground" />
						Roles
					</Card.Title>
					<Card.Description>Permissions this user is granted across applications.</Card.Description>
				</div>
				<Button
					variant="outline"
					size="sm"
					class="shrink-0"
					onclick={() => (assignRoleOpen = true)}
				>
					<PlusIcon />
					Assign
				</Button>
			</div>
		</Card.Header>
		<Card.Content>
			{#if rolesLoading}
				<div class="flex items-center gap-2 text-muted-foreground">
					<LoadingSpinner /> Loading roles…
				</div>
			{:else if rolesQuery.isError}
				<ErrorAlert title="Couldn't load roles" message={rolesQuery.error.message} />
			{:else if roles.length === 0}
				<div
					class="flex flex-col items-center gap-2 rounded-lg border border-dashed p-8 text-center text-muted-foreground"
				>
					<ShieldIcon class="size-7 opacity-60" />
					<p class="text-sm">No roles assigned.</p>
				</div>
			{:else}
				<div class="space-y-2">
					{#each roles as role (role.role_id)}
						<div
							class="group flex items-center gap-3 rounded-lg border p-3 transition hover:bg-muted/50"
						>
							<RoleAvatar {role} />
							<a
								href={`/dashboard/${tenantId}/roles/${role.role_id}`}
								class="min-w-0 flex-1 hover:text-primary"
							>
								<p class="truncate text-sm font-medium">{role.name}</p>
								{#if role.description}
									<p class="truncate text-xs text-muted-foreground">{role.description}</p>
								{:else}
									<p class="truncate text-xs text-muted-foreground">
										{role.user_count} user{role.user_count === 1 ? '' : 's'}
									</p>
								{/if}
							</a>
							<a
								href={`/dashboard/${tenantId}/roles/${role.role_id}`}
								class="shrink-0 text-muted-foreground opacity-0 transition group-hover:opacity-100 hover:text-foreground"
								aria-label="Open role"
							>
								<ExternalLinkIcon class="size-4" />
							</a>
							<Button
								variant="ghost"
								size="icon"
								class="size-7 shrink-0 text-muted-foreground hover:text-destructive"
								disabled={removingRoleId === role.role_id}
								onclick={() => unassignRole(role)}
							>
								{#if removingRoleId === role.role_id}
									<LoadingSpinner />
								{:else}
									<XIcon class="size-4" />
								{/if}
								<span class="sr-only">Unassign role</span>
							</Button>
						</div>
					{/each}
				</div>
			{/if}
		</Card.Content>
	</Card.Root>

	<!-- Groups — live data -->
	<Card.Root>
		<Card.Header>
			<div class="flex items-start justify-between gap-3">
				<div class="space-y-1.5">
					<Card.Title class="flex items-center gap-2">
						<UsersRoundIcon class="size-4 text-muted-foreground" />
						Groups
					</Card.Title>
					<Card.Description>Collections this user belongs to.</Card.Description>
				</div>
				<Button
					variant="outline"
					size="sm"
					class="shrink-0"
					onclick={() => (assignGroupOpen = true)}
				>
					<PlusIcon />
					Add
				</Button>
			</div>
		</Card.Header>
		<Card.Content>
			{#if groupsLoading}
				<div class="flex items-center gap-2 text-muted-foreground">
					<LoadingSpinner /> Loading groups…
				</div>
			{:else if groupsQuery.isError}
				<ErrorAlert title="Couldn't load groups" message={groupsQuery.error.message} />
			{:else if groups.length === 0}
				<div
					class="flex flex-col items-center gap-2 rounded-lg border border-dashed p-8 text-center text-muted-foreground"
				>
					<UsersRoundIcon class="size-7 opacity-60" />
					<p class="text-sm">Not a member of any group.</p>
				</div>
			{:else}
				<div class="space-y-2">
					{#each groups as group (group.group_id)}
						<div
							class="group flex items-center gap-3 rounded-lg border p-3 transition hover:bg-muted/50"
						>
							<GroupAvatar {group} />
							<a
								href={`/dashboard/${tenantId}/groups/${group.group_id}`}
								class="min-w-0 flex-1 hover:text-primary"
							>
								<p class="truncate text-sm font-medium">{group.name}</p>
								{#if group.description}
									<p class="truncate text-xs text-muted-foreground">{group.description}</p>
								{:else}
									<p class="truncate text-xs text-muted-foreground">
										{group.member_count} member{group.member_count === 1 ? '' : 's'}
									</p>
								{/if}
							</a>
							<a
								href={`/dashboard/${tenantId}/groups/${group.group_id}`}
								class="shrink-0 text-muted-foreground opacity-0 transition group-hover:opacity-100 hover:text-foreground"
								aria-label="Open group"
							>
								<ExternalLinkIcon class="size-4" />
							</a>
							<Button
								variant="ghost"
								size="icon"
								class="size-7 shrink-0 text-muted-foreground hover:text-destructive"
								disabled={removingGroupId === group.group_id}
								onclick={() => removeGroup(group)}
							>
								{#if removingGroupId === group.group_id}
									<LoadingSpinner />
								{:else}
									<XIcon class="size-4" />
								{/if}
								<span class="sr-only">Remove from group</span>
							</Button>
						</div>
					{/each}
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
</div>

<AssignRoleDialog
	{tenantId}
	{userId}
	currentRoles={roles}
	open={assignRoleOpen}
	onOpenChange={(v) => (assignRoleOpen = v)}
/>

<AssignGroupDialog
	{tenantId}
	{userId}
	currentGroups={groups}
	open={assignGroupOpen}
	onOpenChange={(v) => (assignGroupOpen = v)}
/>
