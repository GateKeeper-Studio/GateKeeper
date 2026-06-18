<script lang="ts">
	import MailIcon from '@lucide/svelte/icons/mail';

	import type { TenantUserView } from '$lib/services/admin/users';
	import UserAvatar from './user-avatar.svelte';
	import { userFullName } from './display.js';

	let { user, tenantId }: { user: TenantUserView; tenantId: string } = $props();
</script>

<div class="flex items-center gap-3">
	<UserAvatar {user} />
	<div class="min-w-0">
		<a
			href={`/dashboard/${tenantId}/users/${user.user_id}`}
			class="hover:text-primary block truncate font-medium hover:underline"
		>
			{userFullName(user)}
		</a>
		<p class="text-muted-foreground flex items-center gap-1 truncate text-xs">
			<MailIcon class="size-3 shrink-0" />
			<span class="truncate">{user.email}</span>
		</p>
	</div>
</div>
