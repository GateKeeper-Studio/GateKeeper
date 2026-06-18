<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import SearchIcon from '@lucide/svelte/icons/search';
	import ShieldIcon from '@lucide/svelte/icons/shield';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import ErrorAlert from '$lib/components/error-alert.svelte';

	import { listTenantRoles, assignRoleUsers, type TenantRoleView } from '$lib/services/admin/roles';
	import RoleAvatar from '../../roles/role-avatar.svelte';

	type Props = {
		tenantId: string;
		userId: string;
		currentRoles: TenantRoleView[];
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onAssigned?: () => void;
	};

	let { tenantId, userId, currentRoles, open, onOpenChange, onAssigned }: Props = $props();

	const queryClient = useQueryClient();

	const rolesQuery = createQuery(() => ({
		queryKey: ['tenant-roles', tenantId, 'all'],
		enabled: open && !!tenantId,
		queryFn: async () => {
			const [data, err] = await listTenantRoles(tenantId, { pageSize: 500 });
			if (err) throw err;
			return data.items;
		}
	}));

	let search = $state('');
	let selected = $state<Set<string>>(new Set());
	let assigning = $state(false);

	const currentRoleIds = $derived(new Set(currentRoles.map((r) => r.role_id)));

	const candidates = $derived.by(() => {
		const all = rolesQuery.data ?? [];
		const q = search.trim().toLowerCase();
		return all.filter((r) => {
			if (currentRoleIds.has(r.role_id)) return false;
			if (!q) return true;
			return r.name.toLowerCase().includes(q) || (r.description ?? '').toLowerCase().includes(q);
		});
	});

	function toggle(roleId: string) {
		const next = new Set(selected);
		if (next.has(roleId)) {
			next.delete(roleId);
		} else {
			next.add(roleId);
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
		const results = await Promise.all(
			[...selected].map((roleId) => assignRoleUsers(tenantId, roleId, [userId]))
		);
		assigning = false;

		const failed = results.filter(([, err]) => err).length;
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId, userId, 'roles'] });
		queryClient.invalidateQueries({ queryKey: ['tenant-roles', tenantId] });

		if (failed > 0) {
			toast.error(
				`Couldn't assign ${failed} of ${selected.size} role${selected.size === 1 ? '' : 's'}.`
			);
			return;
		}

		const count = selected.size;
		toast.success(`Assigned ${count} role${count === 1 ? '' : 's'}`);
		onAssigned?.();
		handleOpenChange(false);
	}
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>Assign roles</Dialog.Title>
			<Dialog.Description>Select roles from this tenant to assign to this user.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-3">
			<div class="relative">
				<SearchIcon class="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
				<Input placeholder="Search by name…" class="pl-9" bind:value={search} />
			</div>

			{#if rolesQuery.isError}
				<ErrorAlert title="Couldn't load roles" message={rolesQuery.error.message} />
			{:else if rolesQuery.isPending}
				<div class="flex h-32 items-center justify-center gap-2 text-muted-foreground">
					<LoadingSpinner /> Loading roles…
				</div>
			{:else if candidates.length === 0}
				<div class="flex h-32 flex-col items-center justify-center gap-2 text-muted-foreground">
					<ShieldIcon class="size-7 opacity-60" />
					<p class="text-sm">
						{search ? 'No roles match your search.' : 'This user already has every role.'}
					</p>
				</div>
			{:else}
				<div class="max-h-64 overflow-y-auto rounded-lg border">
					{#each candidates as role (role.role_id)}
						<label
							class="flex cursor-pointer items-center gap-3 border-b p-3 last:border-b-0 hover:bg-muted/50"
						>
							<Checkbox
								checked={selected.has(role.role_id)}
								onCheckedChange={() => toggle(role.role_id)}
							/>
							<RoleAvatar {role} />
							<div class="min-w-0">
								<p class="truncate text-sm font-medium">{role.name}</p>
								{#if role.description}
									<p class="truncate text-xs text-muted-foreground">{role.description}</p>
								{/if}
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
				{selected.size > 0
					? `Assign ${selected.size} role${selected.size === 1 ? '' : 's'}`
					: 'Assign roles'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
