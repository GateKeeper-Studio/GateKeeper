<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import SearchIcon from '@lucide/svelte/icons/search';
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';

	import {
		listTenantGroups,
		addGroupMembers,
		type TenantGroupView
	} from '$lib/services/admin/groups';
	import GroupAvatar from '../../groups/group-avatar.svelte';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		tenantId: string;
		userId: string;
		currentGroups: TenantGroupView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onAdded?: () => void;
	};

	let { tenantId, userId, currentGroups, open, onOpenChange, onAdded }: Props = $props();

	const queryClient = useQueryClient();

	const groupsQuery = createQuery(() => ({
		queryKey: ['tenant-groups', tenantId, 'all'],
		enabled: open && !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantGroups(tenantId, { pageSize: 500 });
			if (err) throw err;
			return data.items;
		}
	}));

	let search = $state('');
	let selected = $state<Set<string>>(new Set());
	let adding = $state(false);

	const currentGroupIds = $derived(new Set(currentGroups.map((g) => g.group_id)));

	const candidates = $derived.by(() => {
		const all = groupsQuery.data ?? [];
		const q = search.trim().toLowerCase();
		return all.filter((g) => {
			if (currentGroupIds.has(g.group_id)) return false;
			if (!q) return true;
			return g.name.toLowerCase().includes(q) || (g.description ?? '').toLowerCase().includes(q);
		});
	});

	function toggle(groupId: string) {
		const next = new Set(selected);
		if (next.has(groupId)) {
			next.delete(groupId);
		} else {
			next.add(groupId);
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

		adding = true;
		const results = await Promise.all(
			[...selected].map((groupId) => addGroupMembers(tenantId, groupId, [userId]))
		);
		adding = false;

		const failed = results.filter(([, err]) => err).length;
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId, userId, 'groups'] });
		queryClient.invalidateQueries({ queryKey: ['tenant-groups', tenantId] });

		if (failed > 0) {
			toast.error(
				selected.size === 1
					? m.users_assign_group_error_one({ failed, count: selected.size })
					: m.users_assign_group_error_other({ failed, count: selected.size })
			);
			return;
		}

		const count = selected.size;
		toast.success(
			count === 1
				? m.users_assign_group_success_one({ count })
				: m.users_assign_group_success_other({ count })
		);
		onAdded?.();
		handleOpenChange(false);
	}
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.users_assign_group_dialog_title()}</Dialog.Title>
			<Dialog.Description>{m.users_assign_group_dialog_description()}</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-3">
			<div class="relative">
				<SearchIcon class="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
				<Input placeholder={m.users_search_by_name_placeholder()} class="pl-9" bind:value={search} />
			</div>

			{#if groupsQuery.isError}
				<ErrorAlert title={m.users_access_groups_error_title()} message={groupsQuery.error.message} />
			{:else if groupsQuery.isPending}
				<div class="flex h-32 items-center justify-center gap-2 text-muted-foreground">
					<LoadingSpinner /> {m.users_access_loading_groups()}
				</div>
			{:else if candidates.length === 0}
				<div class="flex h-32 flex-col items-center justify-center gap-2 text-muted-foreground">
					<UsersRoundIcon class="size-7 opacity-60" />
					<p class="text-sm">
						{search ? m.users_assign_group_no_match() : m.users_assign_group_all_added()}
					</p>
				</div>
			{:else}
				<div class="max-h-64 overflow-y-auto rounded-lg border">
					{#each candidates as group (group.group_id)}
						<label
							class="flex cursor-pointer items-center gap-3 border-b p-3 last:border-b-0 hover:bg-muted/50"
						>
							<Checkbox
								checked={selected.has(group.group_id)}
								onCheckedChange={() => toggle(group.group_id)}
							/>
							<GroupAvatar {group} />
							<div class="min-w-0">
								<p class="truncate text-sm font-medium">{group.name}</p>
								{#if group.description}
									<p class="truncate text-xs text-muted-foreground">{group.description}</p>
								{/if}
							</div>
						</label>
					{/each}
				</div>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" disabled={adding} onclick={() => handleOpenChange(false)}>
				{m.users_action_cancel()}
			</Button>
			<Button disabled={selected.size === 0 || adding} onclick={confirm}>
				{#if adding}<LoadingSpinner />{/if}
				{#if selected.size === 0}
					{m.users_assign_group_dialog_title()}
				{:else if selected.size === 1}
					{m.users_assign_group_confirm_one({ count: selected.size })}
				{:else}
					{m.users_assign_group_confirm_other({ count: selected.size })}
				{/if}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
