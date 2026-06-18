<script lang="ts">
	import UsersRoundIcon from '@lucide/svelte/icons/users-round';

	import type { TenantGroupView } from '$lib/services/admin/groups';
	import GroupAvatar from './group-avatar.svelte';

	let { group, tenantId }: { group: TenantGroupView; tenantId: string } = $props();
</script>

<div class="flex items-center gap-3">
	<GroupAvatar {group} />
	<div class="min-w-0">
		<a
			href={`/dashboard/${tenantId}/groups/${group.group_id}`}
			class="hover:text-primary block truncate font-medium hover:underline"
		>
			{group.name}
		</a>
		{#if group.description}
			<p class="text-muted-foreground truncate text-xs">{group.description}</p>
		{:else}
			<p class="text-muted-foreground/50 flex items-center gap-1 truncate text-xs">
				<UsersRoundIcon class="size-3 shrink-0" />
				<span>{group.member_count} member{group.member_count === 1 ? '' : 's'}</span>
			</p>
		{/if}
	</div>
</div>
