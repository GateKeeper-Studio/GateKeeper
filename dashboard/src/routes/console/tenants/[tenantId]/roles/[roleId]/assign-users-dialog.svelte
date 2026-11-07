<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import SearchIcon from '@lucide/svelte/icons/search';
	import UsersIcon from '@lucide/svelte/icons/users';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';

	import { listTenantUsers, type TenantUserView } from '$lib/services/admin/users';
	import { assignRoleUsers } from '$lib/services/admin/roles';
	import { userFullName } from '../../users/display.js';
	import UserAvatar from '../../users/user-avatar.svelte';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		roleId: string;
		currentUsers: TenantUserView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
	};

	let { tenantId, roleId, currentUsers, open, onOpenChange }: Props = $props();

	const queryClient = useQueryClient();

	const usersQuery = createQuery(() => ({
		queryKey: ['tenant-users', tenantId, 'all'],
		enabled: open && !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantUsers(tenantId, { pageSize: 500 });
			if (err) throw err;
			return data.items;
		}
	}));

	let search = $state('');
	let selected = $state<Set<string>>(new Set());
	let assigning = $state(false);

	const currentUserIds = $derived(new Set(currentUsers.map((u) => u.user_id)));

	const candidates = $derived.by(() => {
		const all = usersQuery.data ?? [];
		const q = search.trim().toLowerCase();
		return all.filter((u) => {
			if (currentUserIds.has(u.user_id)) return false;
			if (!q) return true;
			return (
				userFullName(u).toLowerCase().includes(q) ||
				u.email.toLowerCase().includes(q)
			);
		});
	});

	function toggle(userId: string) {
		const next = new Set(selected);
		if (next.has(userId)) {
			next.delete(userId);
		} else {
			next.add(userId);
		}
		selected = next;
	}

	function handleOpenChange(value: boolean) {
		if (!value) {
			search = '';
			selected = new Set();
		}
		onOpenChange(value);
	}

	async function confirm() {
		if (selected.size === 0) return;

		assigning = true;
		const [, err] = await assignRoleUsers(tenantId, roleId, [...selected]);
		assigning = false;

		if (err) {
			toast.error(err.data?.message || m.roles_assign_users_error());
			return;
		}

		const count = selected.size;
		toast.success(count === 1 ? m.roles_assign_users_success_one({ count }) : m.roles_assign_users_success_other({ count }));
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId, roleId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId, roleId, 'users'] });
		handleOpenChange(false);
	}
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.roles_users_assign_button()}</Dialog.Title>
			<Dialog.Description>
				{m.roles_assign_users_description()}
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-3">
			<div class="relative">
				<SearchIcon class="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
				<Input
					placeholder={m.users_search_by_name_placeholder()}
					class="pl-9"
					bind:value={search}
				/>
			</div>

			{#if usersQuery.isError}
				<ErrorAlert title={m.roles_users_load_error_title()} message={usersQuery.error.message} />
			{:else if usersQuery.isPending}
				<div class="text-muted-foreground flex h-32 items-center justify-center gap-2">
					<LoadingSpinner /> {m.roles_assign_users_loading()}
				</div>
			{:else if candidates.length === 0}
				<div class="text-muted-foreground flex h-32 flex-col items-center justify-center gap-2">
					<UsersIcon class="size-7 opacity-60" />
					<p class="text-sm">
						{search ? m.roles_assign_users_no_match() : m.roles_assign_users_all_assigned()}
					</p>
				</div>
			{:else}
				<div class="max-h-64 overflow-y-auto rounded-lg border">
					{#each candidates as user (user.user_id)}
						<label
							class="hover:bg-muted/50 flex cursor-pointer items-center gap-3 border-b p-3 last:border-b-0"
						>
							<Checkbox
								checked={selected.has(user.user_id)}
								onCheckedChange={() => toggle(user.user_id)}
							/>
							<UserAvatar {user} />
							<div class="min-w-0">
								<p class="truncate text-sm font-medium">{userFullName(user)}</p>
								<p class="text-muted-foreground truncate text-xs">{user.email}</p>
							</div>
						</label>
					{/each}
				</div>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" disabled={assigning} onclick={() => handleOpenChange(false)}>
				Cancel
			</Button>
			<Button disabled={selected.size === 0 || assigning} onclick={confirm}>
				{#if assigning}<LoadingSpinner />{/if}
				{selected.size > 0 ? `Assign ${selected.size} user${selected.size === 1 ? '' : 's'}` : 'Assign users'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
