<script lang="ts">
	import ShieldIcon from '@lucide/svelte/icons/shield';

	import type { TenantRoleView } from '$lib/services/admin/roles';
	import RoleAvatar from './role-avatar.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { role, tenantId }: { role: TenantRoleView; tenantId: string } = $props();
</script>

<div class="flex items-center gap-3">
	<RoleAvatar {role} />
	<div class="min-w-0">
		<a
			href={`/tenants/${tenantId}/roles/${role.role_id}`}
			class="hover:text-primary block truncate font-medium hover:underline"
		>
			{role.name}
		</a>
		{#if role.description}
			<p class="text-muted-foreground truncate text-xs">{role.description}</p>
		{:else}
			<p class="text-muted-foreground/50 flex items-center gap-1 truncate text-xs">
				<ShieldIcon class="size-3 shrink-0" />
				<span>
					{role.user_count === 1
						? m.roles_user_count_one({ count: role.user_count })
						: m.roles_user_count_other({ count: role.user_count })}
				</span>
			</p>
		{/if}
	</div>
</div>
